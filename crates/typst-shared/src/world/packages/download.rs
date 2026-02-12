// Added by LDemetrios

use std::collections::BTreeMap;
use std::fmt::Display;
use std::io;
use std::io::Write;
use std::time::{Duration, Instant};

use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;
use serde::{Deserialize, Serialize};
use typst::utils::format_duration;
use typst_kit::downloader::{Progress, ProgressReporter};
use typst_kit::downloader::Downloader;

use crate::export::raw_string::RawString;
use crate::raw_bytes::Base64Bytes;
use crate::terminal::{self, TermOut};

unsafe extern "C" {
    fn download_url(result: &mut RawString, req_len: u64, req_ptr: *mut u8);
}

#[derive(Debug, Serialize, Deserialize)]
struct DownloadRequest {
    url: String,
    headers: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DownloadResponse {
    status: u16,
    headers: BTreeMap<String, String>,
    body: Base64Bytes,
}

type DownloadResult = Result<DownloadResponse, String>;

/// Prints download progress by writing `downloading {0}` followed by repeatedly
/// updating the last terminal line.
pub struct PrintDownload<T>(pub T);

impl<T: Display> ProgressReporter for PrintDownload<T> {
    fn start(&mut self, progress: &Progress) {
        // Print that a package downloading is happening.
        let styles = term::Styles::default();

        let mut out = terminal::out();
        let _ = out.set_color(&styles.header_help);
        let _ = write!(out, "downloading");

        let _ = out.reset();
        let _ = writeln!(out, " {}", self.0);
        self.update(progress);
    }

    fn update(&mut self, progress: &Progress) {
        let mut out = terminal::out();
        let _ = out.clear_last_line();
        let _ = display_download_progress(&mut out, progress);
    }

    fn finish(&mut self, progress: &Progress) {
        self.update(progress);
    }
}

/// Returns a new downloader.
pub fn downloader() -> impl typst_kit::downloader::Downloader {
    let user_agent = concat!("typst/", env!("CARGO_PKG_VERSION"));
    HostDownloader::new(user_agent)
}

#[derive(Debug, Clone)]
struct HostDownloader {
    user_agent: String,
}

impl HostDownloader {
    fn new(user_agent: impl Into<String>) -> Self {
        Self { user_agent: user_agent.into() }
    }
}

impl Downloader for HostDownloader {
    fn stream(
        &self,
        _key: &dyn std::any::Any,
        url: &str,
    ) -> io::Result<(Option<usize>, Box<dyn io::Read>)> {
        let response = self.download_response(url)?;
        if response.status == 404 {
            return Err(io::Error::new(io::ErrorKind::NotFound, "not found"));
        }
        let len = response.body.0.len();
        let reader: Box<dyn io::Read> = Box::new(io::Cursor::new(response.body.0));
        Ok((Some(len), reader))
    }

    fn download(&self, _key: &dyn std::any::Any, url: &str) -> io::Result<Vec<u8>> {
        let response = self.download_response(url)?;
        if response.status == 404 {
            return Err(io::Error::new(io::ErrorKind::NotFound, "not found"));
        }
        Ok(response.body.0)
    }
}

impl HostDownloader {
    fn download_response(&self, url: &str) -> io::Result<DownloadResponse> {
        let mut headers = BTreeMap::new();
        headers.insert("User-Agent".to_string(), self.user_agent.clone());
        let request = DownloadRequest {
            url: url.to_string(),
            headers,
        };
        let req_raw = RawString::from_value(&request);
        let mut res_raw = RawString::default();
        unsafe {
            download_url(&mut res_raw, req_raw.len, req_raw.ptr as *mut u8);
        }
        req_raw.release();
        let result: DownloadResult = res_raw.read_to();
        match result {
            Ok(response) => Ok(response),
            Err(message) => Err(io::Error::new(io::ErrorKind::Other, message)),
        }
    }
}

/// Compile and format several download statistics and make and attempt at
/// displaying them on standard error.
pub fn display_download_progress(
    out: &mut TermOut,
    progress: &Progress,
) -> io::Result<()> {
    let len = progress.samples.len();
    let sum: usize = progress.samples.iter().sum();
    let bytes_per_period =
        if len > 0 { sum / len } else { progress.content_len.unwrap_or(0) };
    let frequency: usize = Duration::from_secs(1)
        .as_nanos()
        .checked_div(progress.period.as_nanos())
        .and_then(|s| s.try_into().ok())
        .unwrap_or(1);
    let speed = bytes_per_period * frequency;

    let total_downloaded = as_bytes_unit(progress.downloaded);
    let speed_h = as_throughput_unit(speed);
    let elapsed = Instant::now().saturating_duration_since(progress.start_time);

    match progress.content_len {
        Some(content_len) => {
            let percent = (progress.downloaded as f64 / content_len as f64) * 100.;
            let remaining = content_len - progress.downloaded;

            let download_size = as_bytes_unit(content_len);
            let eta = Duration::from_secs(if speed == 0 {
                0
            } else {
                (remaining / speed) as u64
            });

            writeln!(
                out,
                "{total_downloaded} / {download_size} ({percent:3.0} %) \
                {speed_h} in {elapsed} ETA: {eta}",
                elapsed = format_duration(elapsed),
                eta = format_duration(eta),
            )?;
        }
        None => writeln!(
            out,
            "Total downloaded: {total_downloaded} \
             Speed: {speed_h} \
             Elapsed: {elapsed}",
            elapsed = format_duration(elapsed),
        )?,
    };
    Ok(())
}

/// Format a given size as a unit of time. Setting `include_suffix` to true
/// appends a '/s' (per second) suffix.
fn as_bytes_unit(size: usize) -> String {
    const KI: f64 = 1024.0;
    const MI: f64 = KI * KI;
    const GI: f64 = KI * KI * KI;

    let size = size as f64;

    if size >= GI {
        format!("{:5.1} GiB", size / GI)
    } else if size >= MI {
        format!("{:5.1} MiB", size / MI)
    } else if size >= KI {
        format!("{:5.1} KiB", size / KI)
    } else {
        format!("{size:3} B")
    }
}

fn as_throughput_unit(size: usize) -> String {
    as_bytes_unit(size) + "/s"
}
