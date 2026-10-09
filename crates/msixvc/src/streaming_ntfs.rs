use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use ntfs_core::{
    parse_attributes,
    parse_attribute_list,
    Attribute,
    AttributeBody,
    MftRecordHeader,
    NtfsError,
    NtfsFs,
    Run,
};

const ATTR_TYPE_ATTRIBUTE_LIST: u32 = 0x20;
const ATTR_TYPE_DATA: u32 = 0x80;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const ROOT_FILE_RECORD_NUMBER: u64 = 5;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NtfsDataRunReport {
    pub start: Option<u64>,
    pub length: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NtfsStreamLayoutReport {
    pub file_record_number: u64,
    pub path: String,
    pub resident_data: bool,
    pub resident_data_length: u64,
    pub value_length: u64,
    pub data_runs: Vec<NtfsDataRunReport>,
}

impl NtfsStreamLayoutReport {
    pub fn is_fragmented_or_embedded(&self) -> bool {
        self.resident_data || self.data_runs.len() > 1
    }
}

pub fn collect_ntfs_stream_layouts<T>(
    fs: &mut T,
) -> ntfs_core::Result<Vec<NtfsStreamLayoutReport>>
where
    T: Read + Seek,
{
    // NtfsFs owns the reader; borrowing it lets us retain the existing API.
    let ntfs = NtfsFs::open(&mut *fs)?;

    let mut reports = Vec::new();
    let mut visited_directories = HashSet::new();

    collect_directory_stream_layouts(
        &ntfs,
        ROOT_FILE_RECORD_NUMBER,
        Path::new(""),
        &mut visited_directories,
        &mut reports,
    )?;

    Ok(reports)
}

pub fn collect_fragmented_or_embedded_ntfs_stream_layouts<T>(
    fs: &mut T,
) -> ntfs_core::Result<Vec<NtfsStreamLayoutReport>>
where
    T: Read + Seek,
{
    let reports = collect_ntfs_stream_layouts(fs)?;

    Ok(reports
        .into_iter()
        .filter(NtfsStreamLayoutReport::is_fragmented_or_embedded)
        .collect())
}

fn collect_directory_stream_layouts<R>(
    ntfs: &NtfsFs<R>,
    directory_record_number: u64,
    base_path: &Path,
    visited_directories: &mut HashSet<u64>,
    reports: &mut Vec<NtfsStreamLayoutReport>,
) -> ntfs_core::Result<()>
where
    R: Read + Seek,
{
    // Prevent cycles in malformed directory indexes.
    if !visited_directories.insert(directory_record_number) {
        return Ok(());
    }

    let directory_record = ntfs.read_record(directory_record_number)?;
    let entries = ntfs.directory_entries(&directory_record)?;

    for entry in entries {
        let Some(file_name) = entry.file_name.as_ref() else {
            continue;
        };

        let name = file_name.name.as_str();

        if name == "." || name == ".." {
            continue;
        }

        let path = join_ntfs_path(base_path, name);
        let child_record_number = entry.file_reference.record_number;

        if file_name.flags & FILE_ATTRIBUTE_DIRECTORY != 0 {
            collect_directory_stream_layouts(
                ntfs,
                child_record_number,
                &path,
                visited_directories,
                reports,
            )?;
        } else {
            collect_file_stream_layouts(
                ntfs,
                child_record_number,
                &path,
                reports,
            )?;
        }
    }

    Ok(())
}

fn parse_record_attributes(
    record: &[u8],
) -> ntfs_core::Result<Vec<Attribute>> {
    let header = MftRecordHeader::parse(record)?;

    parse_attributes(
        record,
        header.first_attribute_offset as usize,
    )
}

pub(crate) fn collect_file_stream_layouts<R>(
    ntfs: &NtfsFs<R>,
    file_record_number: u64,
    path: &Path,
    reports: &mut Vec<NtfsStreamLayoutReport>,
) -> ntfs_core::Result<()>
where
    R: Read + Seek,
{
    let record = ntfs.read_record(file_record_number)?;
    let attributes = parse_record_attributes(&record)?;

    // Cache attributes by MFT record so we can also discover $DATA streams
    // referenced by a resident $ATTRIBUTE_LIST.
    let mut attribute_cache: HashMap<u64, Vec<Attribute>> =
        HashMap::new();

    attribute_cache.insert(file_record_number, attributes.clone());

    // Each candidate contains its starting VCN and parsed attribute.
    // The lowest VCN carries the authoritative stream length.
    let mut candidates: Vec<(u64, Attribute)> = Vec::new();

    // An attribute ID is unique within an MFT record.
    let mut seen_attributes: HashSet<(u64, u16)> =
        HashSet::new();

    for attribute in &attributes {
        if attribute.type_code != ATTR_TYPE_DATA {
            continue;
        }

        seen_attributes.insert((
            file_record_number,
            attribute.attribute_id,
        ));

        let start_vcn = match &attribute.body {
            AttributeBody::Resident { .. } => 0,
            AttributeBody::NonResident { start_vcn, .. } => {
                *start_vcn
            }
        };

        candidates.push((start_vcn, attribute.clone()));
    }

    // The high-level NTFS crate resolves extension records when a stream
    // name is known. A resident $ATTRIBUTE_LIST also lets us discover
    // named streams whose attributes live entirely in extension records.
    if let Some(list_content) = attributes
        .iter()
        .find(|a| a.type_code == ATTR_TYPE_ATTRIBUTE_LIST)
        .and_then(|a| a.resident_content(&record))
    {
        let entries = parse_attribute_list(list_content)?;

        for entry in entries {
            if entry.type_code != ATTR_TYPE_DATA {
                continue;
            }

            let source_record_number =
                entry.base_reference.record_number;

            if !seen_attributes.insert((
                source_record_number,
                entry.attribute_id,
            )) {
                continue;
            }

            if !attribute_cache.contains_key(&source_record_number) {
                let source_record =
                    ntfs.read_record(source_record_number)?;

                let source_attributes =
                    parse_record_attributes(&source_record)?;

                attribute_cache.insert(
                    source_record_number,
                    source_attributes,
                );
            }

            let referenced_attribute = attribute_cache
                .get(&source_record_number)
                .and_then(|source_attributes| {
                    source_attributes.iter().find(|attribute| {
                        attribute.type_code == entry.type_code
                            && attribute.attribute_id
                                == entry.attribute_id
                            && attribute.name == entry.name
                    })
                })
                .cloned()
                .ok_or(NtfsError::BadAttributeList(
                    "referenced $DATA attribute not found",
                ))?;

            candidates.push((
                entry.start_vcn,
                referenced_attribute,
            ));
        }
    }

    // Process VCN 0 first so a later extension fragment cannot accidentally
    // supply the size of the complete stream.
    candidates.sort_by_key(|(start_vcn, _)| *start_vcn);

    let mut reported_streams: HashSet<Option<String>> =
        HashSet::new();

    let cluster_size = ntfs.boot().cluster_size();

    for (_, attribute) in candidates {
        let stream_name = attribute.name.clone();

        // Multiple attribute-list entries can describe fragments of the
        // same stream. Emit one report per stream, not one per fragment.
        if !reported_streams.insert(stream_name.clone()) {
            continue;
        }

        let report_path = match stream_name.as_deref() {
            Some(name) => format!("{}:{}", path.display(), name),
            None => path.display().to_string(),
        };

        let report = match &attribute.body {
            AttributeBody::Resident {
                content_length,
                ..
            } => {
                let length = u64::from(*content_length);

                NtfsStreamLayoutReport {
                    file_record_number,
                    path: report_path,
                    resident_data: length != 0,
                    resident_data_length: length,
                    value_length: length,
                    data_runs: Vec::new(),
                }
            }

            AttributeBody::NonResident {
                real_size,
                ..
            } => {
                // Unlike the old NtfsDataRun iterator, ntfs-core returns
                // run lengths in clusters and LCNs rather than byte offsets.
                let runs = ntfs.runs_by_record(
                    file_record_number,
                    stream_name.as_deref(),
                )?;

                let data_runs = runs
                    .into_iter()
                    .map(|run| {
                        data_run_report_from_run(
                            run,
                            cluster_size,
                        )
                    })
                    .collect::<ntfs_core::Result<Vec<_>>>()?;

                NtfsStreamLayoutReport {
                    file_record_number,
                    path: report_path,
                    resident_data: false,
                    resident_data_length: 0,
                    value_length: *real_size,
                    data_runs,
                }
            }
        };

        reports.push(report);
    }

    Ok(())
}

fn data_run_report_from_run(
    run: Run,
    cluster_size: u64,
) -> ntfs_core::Result<NtfsDataRunReport> {
    let start = match run.lcn {
        Some(lcn) => Some(
            lcn.checked_mul(cluster_size)
                .ok_or(NtfsError::BadRunlist(
                    "data-run byte offset overflow",
                ))?,
        ),
        None => None,
    };

    let length = run
        .length
        .checked_mul(cluster_size)
        .ok_or(NtfsError::BadRunlist(
            "data-run byte length overflow",
        ))?;

    Ok(NtfsDataRunReport { start, length })
}

fn join_ntfs_path(base_path: &Path, name: &str) -> PathBuf {
    if base_path.as_os_str().is_empty() {
        PathBuf::from(name)
    } else {
        base_path.join(name)
    }
}