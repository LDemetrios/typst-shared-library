// Added by LDemetrios

use crate::compile::diagnostics_from_message;
use crate::extended_info::ExtendedSourceDiagnostic;
use crate::raw_string::RawString;
use serde::Deserialize;
use std::num::NonZeroUsize;
use typst_library::foundations::Smart;
use typst_library::layout::PageRanges;
use typst_pdf::{PdfOptions, PdfStandard, PdfStandards, Timestamp};

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PdfOptionsConfig {
    ident: Option<PdfIdentConfig>,
    timestamp: Option<PdfTimestampConfig>,
    page_ranges: Option<Vec<PdfPageRangeConfig>>,
    standards: Option<PdfStandardsConfig>,
    tagged: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum PdfIdentConfig {
    Auto,
    Custom { value: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PdfTimestampConfig {
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    timezone: Option<PdfTimezoneConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum PdfTimezoneConfig {
    Utc,
    Local {
        #[serde(rename = "hourOffset")]
        hour_offset: i8,
        #[serde(rename = "minuteOffset")]
        minute_offset: u8,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PdfPageRangeConfig {
    start: Option<u32>,
    end: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PdfStandardsConfig {
    validator: Option<PdfValidatorConfig>,
    version: Option<PdfVersionConfig>,
}

#[derive(Debug, Deserialize)]
enum PdfValidatorConfig {
    None,
    A1_A,
    A1_B,
    A2_A,
    A2_B,
    A2_U,
    A3_A,
    A3_B,
    A3_U,
    UA1,
    A4,
    A4F,
    A4E,
}

#[derive(Debug, Deserialize)]
enum PdfVersionConfig {
    Pdf14,
    Pdf15,
    Pdf16,
    Pdf17,
    Pdf20,
}

pub(crate) fn parse_pdf_options<'a>(
    raw: RawString,
) -> Result<PdfOptions<'a>, Vec<ExtendedSourceDiagnostic>> {
    let config = match parse_pdf_options_config( raw) {
        Ok(config) => config,
        Err(diagnostics) => {
            return  Err(diagnostics);
        }
    };

    let mut options = PdfOptions::default();
    options.ident = Smart::Auto;

    if let Some(timestamp) = config.timestamp {
        options.timestamp = Some(match parse_pdf_timestamp(timestamp) {
            Ok(value) => value,
            Err(diagnostics) => {
                return Err(diagnostics);
            }
        });
    }

    if let Some(ranges) = config.page_ranges {
        options.page_ranges = Some(parse_page_ranges(ranges));
    }

    if let Some(standards) = config.standards {
        options.standards = match parse_standards(standards) {
            Ok(value) => value,
            Err(diagnostics) => {
                return  Err(diagnostics);
            }
        };
    }

    if let Some(tagged) = config.tagged {
        options.tagged = tagged;
    }
     Ok(options)
}

fn parse_pdf_timestamp(
    config: PdfTimestampConfig,
) -> Result<Timestamp, Vec<ExtendedSourceDiagnostic>> {
    let datetime = typst_library::foundations::Datetime::from_ymd_hms(
        config.year,
        config.month,
        config.day,
        config.hour,
        config.minute,
        config.second,
    )
        .ok_or_else(|| diagnostics_from_message("Invalid PDF timestamp datetime"))?;

    match config.timezone {
        Some(PdfTimezoneConfig::Utc) => Ok(Timestamp::new_utc(datetime)),
        Some(PdfTimezoneConfig::Local { hour_offset, minute_offset }) => {
            let offset = (hour_offset as i32) * 60 + (minute_offset as i32);
            Timestamp::new_local(datetime, offset).ok_or_else(|| {
                diagnostics_from_message( "Invalid PDF timestamp timezone offset")
            })
        }
        None => Ok(Timestamp::new_utc(datetime)),
    }
}

fn parse_page_ranges(ranges: Vec<PdfPageRangeConfig>) -> PageRanges {
    PageRanges::new(
        ranges
            .iter()
            .map(|range| {
                let start =
                    range.start.and_then(|value| NonZeroUsize::new((value + 1) as usize));
                let end =
                    range.end.and_then(|value| NonZeroUsize::new((value + 1) as usize));

                start..=end
            })
            .collect(),
    )
}

fn parse_standards(
    config: PdfStandardsConfig,
) -> Result<PdfStandards, Vec<ExtendedSourceDiagnostic>> {
    let mut list = Vec::new();
    if let Some(version) = config.version {
        let standard = match version {
            PdfVersionConfig::Pdf14 => PdfStandard::V_1_4,
            PdfVersionConfig::Pdf15 => PdfStandard::V_1_5,
            PdfVersionConfig::Pdf16 => PdfStandard::V_1_6,
            PdfVersionConfig::Pdf17 => PdfStandard::V_1_7,
            PdfVersionConfig::Pdf20 => PdfStandard::V_2_0,
        };
        list.push(standard);
    }

    if let Some(validator) = config.validator {
        let standard = match validator {
            PdfValidatorConfig::None => None,
            PdfValidatorConfig::A1_A => Some(PdfStandard::A_1a),
            PdfValidatorConfig::A1_B => Some(PdfStandard::A_1b),
            PdfValidatorConfig::A2_A => Some(PdfStandard::A_2a),
            PdfValidatorConfig::A2_B => Some(PdfStandard::A_2b),
            PdfValidatorConfig::A2_U => Some(PdfStandard::A_2u),
            PdfValidatorConfig::A3_A => Some(PdfStandard::A_3a),
            PdfValidatorConfig::A3_B => Some(PdfStandard::A_3b),
            PdfValidatorConfig::A3_U => Some(PdfStandard::A_3u),
            PdfValidatorConfig::UA1 => Some(PdfStandard::Ua_1),
            PdfValidatorConfig::A4 => Some(PdfStandard::A_4),
            PdfValidatorConfig::A4F => Some(PdfStandard::A_4f),
            PdfValidatorConfig::A4E => Some(PdfStandard::A_4e),
        };
        if let Some(standard) = standard {
            list.push(standard);
        }
    }

    if list.is_empty() {
        return Ok(PdfStandards::default());
    }

    PdfStandards::new(&list)
        .map_err(|err| diagnostics_from_message(err.to_string()))
}

fn parse_pdf_options_config(
    options: RawString,
) -> Result<PdfOptionsConfig, Vec<ExtendedSourceDiagnostic>> {
    let mut config: PdfOptionsConfig = PdfOptionsConfig::default();
    let text = options.as_str().unwrap();
    if !text.is_empty() {
        config = serde_json::from_str(text)
            .map_err(|err| diagnostics_from_message(err.to_string()))?;
    }

    Ok(config)
}
