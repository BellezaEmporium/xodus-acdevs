use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::vec;

use fs2::available_space;
use futures_util::{StreamExt, stream};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use msixvc::streaming;
use msixvc::xvd::{SegmentFile, XvdFile};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncRead;
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;
use xodus::tokens::TokenManager;

use crate::license::get_license;
use crate::package::{get_content_id, get_packages};

/// Errors from every layer (HTTP, filesystem, msixvc parsing, licensing)
/// collapsed into one printable type.
type BoxError = Box<dyn std::error::Error + Send + Sync>;

struct Job {
    name: String,
    content: SegmentFile,
}

enum ProgressEvent {
    Started { id: usize, name: String, total: u64 },
    Advanced { id: usize, delta: u64 },
    Finished { id: usize },
    UpdateRemaining { name: String, total: u64 },
    UpdateStatus { name: String },
}

pub async fn run(
    client: &reqwest::Client,
    tokens: &TokenManager,
    source: String,
    destination: String,
    try_skip_ntfs: bool,
    parallel: Option<usize>,
    market: Option<String>,
) -> ExitCode {
    let (tx, rx) = tokio::sync::mpsc::channel::<ProgressEvent>(256);
    let reader_result: Result<(), BoxError> = if source.starts_with("file://") {
        let fsrc = source.strip_prefix("file://").unwrap_or_default();
        let f = match File::open(fsrc).await {
            Ok(f) => f,
            Err(err) => {
                eprintln!("could not open {fsrc}: {err}");
                return ExitCode::FAILURE;
            }
        };
        let l = match f.metadata().await {
            Ok(metadata) => metadata.len(),
            Err(err) => {
                eprintln!("could not read metadata for {fsrc}: {err}");
                return ExitCode::FAILURE;
            }
        };
        run_cli_reader(
            client,
            tokens,
            destination,
            try_skip_ntfs,
            parallel,
            market,
            f,
            l,
            &source,
            &tx,
            rx,
        )
        .await
    } else {
        let vurl = if source.starts_with("http://") || source.starts_with("https://") {
            source
        } else {
            let content_id = if Uuid::try_parse(&source).is_err() {
                match get_content_id(client, source, market.clone()).await {
                    Ok(id) => id,
                    Err(err) => {
                        eprintln!("could not resolve product title: {err}");
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                source
            };
            let package = match get_packages(client, tokens, content_id.clone()).await {
                Ok(p) => p,
                Err(err) => {
                    eprintln!("could not list packages: {err}");
                    return ExitCode::FAILURE;
                }
            };
            let Some(file) = package
                .package_files
                .iter()
                .find(|p| p.file_name.ends_with(".msixvc"))
            else {
                eprintln!("No .msixvc file found");
                return ExitCode::FAILURE;
            };
            let Some(cdn_root) = file.cdn_root_paths.first() else {
                eprintln!("package listing contains no CDN root paths");
                return ExitCode::FAILURE;
            };
            format!("{cdn_root}{}", file.relative_url)
        };
        let url = &vurl;
        let mut pos = 0;
        let http_file = match streaming::HttpRead::open(
            client.clone(),
            url,
            Some(|c, _| {
                if tx
                    .try_send(ProgressEvent::Advanced {
                        id: usize::MAX,
                        delta: c - pos,
                    })
                    .is_ok()
                {
                    pos = c;
                }
            }),
        )
        .await
        {
            Ok(f) => f,
            Err(err) => {
                eprintln!("could not open remote package {url}: {err}");
                return ExitCode::FAILURE;
            }
        };
        let l = http_file.len();

        run_cli_reader(
            client,
            tokens,
            destination,
            try_skip_ntfs,
            parallel,
            market,
            http_file,
            l,
            url,
            &tx,
            rx,
        )
        .await
    };

    match reader_result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("streaming failed: {err}");
            ExitCode::FAILURE
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_cli_reader<Reader>(
    client: &reqwest::Client,
    tokens: &TokenManager,
    destination: String,
    try_skip_ntfs: bool,
    parallel: Option<usize>,
    market: Option<String>,
    reader: Reader,
    l: u64,
    url: &str,
    tx: &Sender<ProgressEvent>,
    mut rx: Receiver<ProgressEvent>,
) -> Result<(), BoxError>
where
    Reader: AsyncRead + Unpin,
{
    tokio::spawn(async move {
        let multi_progress = MultiProgress::new();
        let total_progess = multi_progress.add(ProgressBar::new(l).with_style(
            ProgressStyle::with_template("{msg:30!} {bytes:>12}/{total_bytes:>12} {bytes_per_sec:>12} [{bar:40.cyan/blue}] {percent:>3}%").unwrap()
            .progress_chars("#>-")
        ));

        total_progess.set_message("Initializing");
        let mut bars: HashMap<usize, ProgressBar> = HashMap::new();

        while let Some(event) = rx.recv().await {
            match event {
                ProgressEvent::Started { id, name, total } => {
                    let cur_progess = multi_progress.add(ProgressBar::new(total).with_style(
                        ProgressStyle::with_template("{msg:30!} {bytes:>12}/{total_bytes:>12} {bytes_per_sec:>12} [{bar:40.cyan/blue}] {percent:>3}%").unwrap()
                        .progress_chars("#>-")
                    ));
                    cur_progess.set_message(name);
                    bars.insert(id, cur_progess);
                }
                ProgressEvent::Advanced { id, delta } => {
                    if let Some(bar) = bars.get(&id) {
                        bar.inc(delta);
                    }
                    total_progess.inc(delta);
                }
                ProgressEvent::Finished { id } => {
                    if let Some(bar) = bars.remove(&id) {
                        bar.finish_and_clear();
                    }
                }
                ProgressEvent::UpdateRemaining { name, total } => {
                    total_progess.set_message(name);
                    total_progess.set_length(total_progess.position() + total);
                }
                ProgressEvent::UpdateStatus { name } => {
                    total_progess.set_message(name);
                }
            }
        }

        total_progess.abandon();
    });
    run_reader(
        client,
        tokens,
        destination,
        try_skip_ntfs,
        parallel,
        market,
        reader,
        l,
        url,
        tx,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_reader<Reader>(
    client: &reqwest::Client,
    tokens: &TokenManager,
    destination: String,
    try_skip_ntfs: bool,
    parallel: Option<usize>,
    market: Option<String>,
    reader: Reader,
    l: u64,
    url: &str,
    tx: &Sender<ProgressEvent>,
) -> Result<(), BoxError>
where
    Reader: AsyncRead + Unpin,
{
    let out: &Path = Path::new(&destination);

    std::fs::create_dir_all(out).map_err(|e| {
        format!(
            "could not create destination directory {}: {e}",
            out.display()
        )
    })?;

    let cache_path = out.join(".xodus-streaming-tmp.msixvc");
    let final_path = out.join(".xodus-streaming.msixvc");

    let mut remote_file = streaming::PrefixCacheFile::new(reader, l, cache_path.clone()).await?;
    let remote_xvd = XvdFile::parse(&mut remote_file)
        .await
        .map_err(|e| format!("could not parse remote package header: {e}"))?;
    let mut rfiles: HashMap<String, SegmentFile> = HashMap::new();
    let mut lfiles: HashMap<String, SegmentFile> = HashMap::new();

    let files = remote_xvd
        .parse_user_package_files(&mut remote_file)
        .await
        .map_err(|e| format!("could not parse package file list: {e}"))?;
    for (k, v) in &files {
        if k == "SegmentMetadata.bin" {
            rfiles = remote_xvd
                .parse_segment_metadata(&mut remote_file, v)
                .await
                .map_err(|e| format!("could not parse segment metadata: {e}"))?;
        }
    }

    if !try_skip_ntfs || rfiles.is_empty() {
        tx.send(ProgressEvent::UpdateStatus {
            name: "Downloading ntfs...".to_owned(),
        })
        .await
        .ok();
        let sfiles = remote_xvd
            .parse_ntfs_segment_metadata(&mut remote_file, !rfiles.is_empty())
            .await
            .map_err(|e| format!("could not parse NTFS segment metadata: {e}"))?;
        rfiles.extend(sfiles);
    }

    let file = OpenOptions::new()
        .read(true)
        .open(final_path.to_owned())
        .await
        .ok();

    if let Some(mut file) = file {
        let xvd = XvdFile::parse(&mut file)
            .await
            .map_err(|e| format!("could not parse existing local download: {e}"))?;

        let files = xvd
            .parse_user_package_files(&mut file)
            .await
            .map_err(|e| format!("could not parse local package file list: {e}"))?;
        for (k, v) in &files {
            if k == "SegmentMetadata.bin" {
                lfiles = xvd
                    .parse_segment_metadata(&mut file, v)
                    .await
                    .map_err(|e| format!("could not parse local segment metadata: {e}"))?;
            }
        }

        if let Ok(sfiles) = xvd
            .parse_ntfs_segment_metadata(&mut file, !lfiles.is_empty())
            .await
        {
            lfiles.extend(sfiles);
        }
    }

    let license = get_license(
        client,
        tokens,
        remote_xvd.content_id().to_string(),
        market.unwrap_or("neutral".to_string()),
    )
    .await?;
    let (key, game_splicense) = license;
    if game_splicense.content_keys.len() != 1 {
        return Err(format!(
            "unexpected number of content keys {}",
            game_splicense.content_keys.len()
        )
        .into());
    }
    let Some((_, content_key)) = game_splicense.content_keys.into_iter().next() else {
        return Err("license carries no content key".into());
    };

    let full_key = content_key
        .unpack(&key)
        .map_err(|e| format!("failed to unpack content key: {e}"))?;

    let total_size: u64 = rfiles
        .iter()
        .filter(|(k, v1)| {
            if let Some(v2) = lfiles.get(*k) {
                v1.data_hashs != v2.data_hashs || v1.data_hashs.is_empty()
            } else {
                true
            }
        })
        .map(|(_, v)| v.length)
        .sum();

    let required_free_space = total_size;
    let available_free_space = match available_space(out) {
        Ok(space) => space,
        Err(err) => {
            return Err(format!(
                "failed to determine available space for {}: {err}",
                out.display()
            )
            .into());
        }
    };

    if available_free_space < required_free_space {
        return Err(format!(
            "not enough free disk space on {}: need {} bytes, have {} bytes (files: {})",
            out.display(),
            required_free_space,
            available_free_space,
            total_size
        )
        .into());
    }

    tx.send(ProgressEvent::UpdateRemaining {
        name: "Downloading".to_owned(),
        total: total_size,
    })
    .await
    .ok();

    let remote_xvd_ref = &remote_xvd;
    let failed = AtomicUsize::new(0);
    stream::iter(
        rfiles
            .iter()
            .filter(|(k, v1)| {
                if let Some(v2) = lfiles.get(*k) {
                    v1.data_hashs != v2.data_hashs || v1.data_hashs.is_empty()
                } else {
                    true
                }
            })
            .map(|(n, v)| Job {
                name: n.clone(),
                content: SegmentFile {
                    offset: v.offset,
                    length: v.length,
                    data_hashs: vec![],
                    keep_encrypted: v.keep_encrypted,
                },
            })
            .enumerate(),
    )
    .for_each_concurrent(parallel.unwrap_or(4), |(id, job)| {
        let tx = tx.clone();
        let client = client.clone();
        let failed = &failed;
        async move {
            // One failing segment must not abort the whole download: report
            // it, keep the remaining segments going, and fail the command at
            // the end so the user sees a non-zero exit code.
            if let Err(err) =
                process_segment(remote_xvd_ref, &client, url, out, &job, *full_key, &tx, id).await
            {
                eprintln!("segment {} failed: {err}", job.name);
                failed.fetch_add(1, Ordering::Relaxed);
            }
            tx.send(ProgressEvent::Finished { id }).await.ok();
        }
    })
    .await;

    let failed_count = failed.load(Ordering::Relaxed);
    if failed_count > 0 {
        return Err(format!("{failed_count} segment(s) failed to download").into());
    }

    std::fs::remove_file(&final_path).ok();
    std::fs::rename(&cache_path, &final_path).map_err(|e| {
        format!(
            "could not finalize download (rename {} -> {}): {e}",
            cache_path.display(),
            final_path.display()
        )
    })?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn process_segment(
    xvd: &XvdFile,
    client: &reqwest::Client,
    url: &str,
    out: &Path,
    job: &Job,
    key: [u8; 32],
    tx: &Sender<ProgressEvent>,
    id: usize,
) -> Result<(), BoxError> {
    let target_file = out.join(job.name.replace("\\", "/"));
    if let Some(folder) = target_file.parent() {
        std::fs::create_dir_all(folder)
            .map_err(|e| format!("could not create directory {}: {e}", folder.display()))?;
    }
    let mut fout = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&target_file)
        .await
        .map_err(|e| format!("could not open {}: {e}", target_file.display()))?;
    let mut lp = 0;

    let progress = |pos, _: u64| {
        if tx
            .try_send(ProgressEvent::Advanced {
                id,
                delta: pos - lp,
            })
            .is_ok()
        {
            lp = pos;
        }
    };
    // Unicode-safe truncation: slicing bytes can split a multi-byte UTF-8
    // sequence and panic on non-ASCII file names.
    let path = job.name.to_owned();
    let char_count = path.chars().count();
    let shown = if char_count > 30 {
        let tail: String = path.chars().skip(char_count - 27).collect();
        format!("...{tail}")
    } else {
        path.clone()
    };
    tx.send(ProgressEvent::Started {
        id,
        name: shown,
        total: job.content.length,
    })
    .await
    .ok();

    if let Some(fpath) = url.strip_prefix("file://") {
        let mut i = File::open(fpath)
            .await
            .map_err(|e| format!("could not open {fpath}: {e}"))?;
        // msixvc returns a non-Send `Box<dyn Error>`; stringify so the error
        // can cross the concurrent-download future boundary.
        xvd.extract_file(&mut i, &mut fout, &job.content, key, progress)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        xvd.download_file_http(client, url, &mut fout, &job.content, key, progress)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
