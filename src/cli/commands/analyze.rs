//! Combined stock overview command for agent consumers.

use serde::Serialize;
use tracing::instrument;

use crate::cli::common::auth::make_client;
use crate::cli::common::error::{CliError, structured_client_error, structured_no_results_error};
use crate::cli::output::{finish_output, print_json};
use crate::cli::{AnalyzeArgs, AnalyzeSectionArg};

use super::fundamentals::{FundamentalsRecord, flatten_fundamentals};
use super::industry::{IndustryRsRecord, flatten_industry_rs};
use super::market_data::{MarketDataRecord, flatten_market_data};
use super::ownership::{OwnershipSummaryRecord, flatten_ownership_summary};
use super::ratings::{RatingsRecord, flatten_ratings};

const SNAPSHOT_NO_DATA: &str = "snapshot section returned no data for symbol";
const RATINGS_NO_DATA: &str = "ratings section returned no data for symbol";
const FUNDAMENTALS_NO_DATA: &str = "fundamentals section returned no data for symbol";
const INDUSTRY_NO_DATA: &str = "industry section returned no data for symbol";
const OWNERSHIP_NO_DATA: &str = "ownership section returned no data for symbol";

#[derive(Debug, Clone, Copy)]
struct SectionSelection {
    snapshot: bool,
    ratings: bool,
    fundamentals: bool,
    industry: bool,
    ownership: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
enum AnalyzeSection<T> {
    Data(T),
    Error(CliError),
}

#[derive(Debug, Clone)]
enum SectionSource<T> {
    Data(Vec<T>),
    Error(CliError),
}

#[derive(Debug, Clone, Serialize)]
struct AnalyzeRecord {
    symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<AnalyzeSection<MarketDataRecord>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ratings: Option<AnalyzeSection<Vec<RatingsRecord>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fundamentals: Option<AnalyzeSection<Vec<FundamentalsRecord>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    industry: Option<AnalyzeSection<IndustryRsRecord>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ownership: Option<AnalyzeSection<Vec<OwnershipSummaryRecord>>>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum AnalyzeOutput {
    Single(Box<AnalyzeRecord>),
    Multiple(Vec<AnalyzeRecord>),
}

/// Handles the analyze command.
#[instrument(skip_all)]
#[cfg(not(coverage))]
pub async fn handle(args: &AnalyzeArgs, fields: &[String]) -> i32 {
    let client = match make_client().await {
        Ok(client) => client,
        Err(code) => return code,
    };

    let symbol_refs = args
        .symbols
        .symbols
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let selection = SectionSelection::from_sections(&args.sections);

    let snapshot_selected = selection.snapshot;
    let ratings_selected = selection.ratings;
    let fundamentals_selected = selection.fundamentals;
    let industry_selected = selection.industry;
    let ownership_selected = selection.ownership;

    let (snapshot, ratings, fundamentals, industry, ownership) = tokio::join!(
        async {
            if snapshot_selected {
                Some(
                    client
                        .other_market_data(
                            &symbol_refs,
                            "CHARTING",
                            "P12Q_AGO",
                            "P24Q_AGO",
                            "P4Q_FUTURE",
                        )
                        .await,
                )
            } else {
                None
            }
        },
        async {
            if ratings_selected {
                Some(client.rs_rating_ri_panel(&symbol_refs, None).await)
            } else {
                None
            }
        },
        async {
            if fundamentals_selected {
                Some(
                    client
                        .fundamentals(
                            &symbol_refs,
                            "CHARTING",
                            "P7Y_AGO",
                            "P2Y_FUTURE",
                            "P7Y_AGO",
                            "P2Y_FUTURE",
                        )
                        .await,
                )
            } else {
                None
            }
        },
        async {
            if industry_selected {
                Some(client.industry_group_rs(&symbol_refs, None).await)
            } else {
                None
            }
        },
        async {
            if ownership_selected {
                Some(client.ownership(&symbol_refs).await)
            } else {
                None
            }
        }
    );

    let snapshot = snapshot.map(|result| match result {
        Ok(response) => {
            SectionSource::Data(flatten_market_data(&symbol_refs, &response.market_data))
        }
        Err(err) => SectionSource::Error(structured_client_error(&err)),
    });
    let ratings = ratings.map(|result| match result {
        Ok(response) => SectionSource::Data(flatten_ratings(&symbol_refs, response)),
        Err(err) => SectionSource::Error(structured_client_error(&err)),
    });
    let fundamentals = fundamentals.map(|result| match result {
        Ok(response) => {
            SectionSource::Data(flatten_fundamentals(&symbol_refs, &response.market_data))
        }
        Err(err) => SectionSource::Error(structured_client_error(&err)),
    });
    let industry = industry.map(|result| match result {
        Ok(response) => {
            SectionSource::Data(flatten_industry_rs(&symbol_refs, &response.market_data))
        }
        Err(err) => SectionSource::Error(structured_client_error(&err)),
    });
    let ownership = ownership.map(|result| match result {
        Ok(response) => SectionSource::Data(flatten_ownership_summary(
            &symbol_refs,
            &response.market_data,
        )),
        Err(err) => SectionSource::Error(structured_client_error(&err)),
    });

    let records = analyze_records(
        &symbol_refs,
        snapshot.as_ref(),
        ratings.as_ref(),
        fundamentals.as_ref(),
        industry.as_ref(),
        ownership.as_ref(),
    );
    let output = analyze_output(records);

    finish_output(print_json(&output, fields))
}

impl SectionSelection {
    fn all() -> Self {
        Self {
            snapshot: true,
            ratings: true,
            fundamentals: true,
            industry: true,
            ownership: true,
        }
    }

    fn from_sections(sections: &[AnalyzeSectionArg]) -> Self {
        if sections.is_empty() {
            return Self::all();
        }

        Self {
            snapshot: sections.contains(&AnalyzeSectionArg::Snapshot),
            ratings: sections.contains(&AnalyzeSectionArg::Ratings),
            fundamentals: sections.contains(&AnalyzeSectionArg::Fundamentals),
            industry: sections.contains(&AnalyzeSectionArg::Industry),
            ownership: sections.contains(&AnalyzeSectionArg::Ownership),
        }
    }
}

fn analyze_output(mut records: Vec<AnalyzeRecord>) -> AnalyzeOutput {
    if records.len() == 1 {
        AnalyzeOutput::Single(Box::new(records.remove(0)))
    } else {
        AnalyzeOutput::Multiple(records)
    }
}

fn analyze_records(
    symbols: &[&str],
    snapshot: Option<&SectionSource<MarketDataRecord>>,
    ratings: Option<&SectionSource<RatingsRecord>>,
    fundamentals: Option<&SectionSource<FundamentalsRecord>>,
    industry: Option<&SectionSource<IndustryRsRecord>>,
    ownership: Option<&SectionSource<OwnershipSummaryRecord>>,
) -> Vec<AnalyzeRecord> {
    symbols
        .iter()
        .map(|symbol| AnalyzeRecord {
            symbol: (*symbol).to_string(),
            snapshot: snapshot.map(|source| {
                section_one_for_symbol(source, symbol, |record| &record.symbol, SNAPSHOT_NO_DATA)
            }),
            ratings: ratings.map(|source| {
                section_vec_for_symbol(source, symbol, |record| &record.symbol, RATINGS_NO_DATA)
            }),
            fundamentals: fundamentals.map(|source| {
                section_vec_for_symbol(
                    source,
                    symbol,
                    |record| &record.symbol,
                    FUNDAMENTALS_NO_DATA,
                )
            }),
            industry: industry.map(|source| {
                section_one_for_symbol(source, symbol, |record| &record.symbol, INDUSTRY_NO_DATA)
            }),
            ownership: ownership.map(|source| {
                section_vec_for_symbol(source, symbol, |record| &record.symbol, OWNERSHIP_NO_DATA)
            }),
        })
        .collect()
}

fn section_one_for_symbol<T, F>(
    source: &SectionSource<T>,
    symbol: &str,
    symbol_of: F,
    no_data_message: &'static str,
) -> AnalyzeSection<T>
where
    T: Clone,
    F: Fn(&T) -> &str,
{
    match source {
        SectionSource::Data(records) => records
            .iter()
            .find(|record| symbol_of(record) == symbol)
            .cloned()
            .map(AnalyzeSection::Data)
            .unwrap_or_else(|| AnalyzeSection::Error(structured_no_results_error(no_data_message))),
        SectionSource::Error(error) => AnalyzeSection::Error(error.clone()),
    }
}

fn section_vec_for_symbol<T, F>(
    source: &SectionSource<T>,
    symbol: &str,
    symbol_of: F,
    no_data_message: &'static str,
) -> AnalyzeSection<Vec<T>>
where
    T: Clone,
    F: Fn(&T) -> &str,
{
    match source {
        SectionSource::Data(records) => {
            let selected = records
                .iter()
                .filter(|record| symbol_of(record) == symbol)
                .cloned()
                .collect::<Vec<_>>();

            if selected.is_empty() {
                AnalyzeSection::Error(structured_no_results_error(no_data_message))
            } else {
                AnalyzeSection::Data(selected)
            }
        }
        SectionSource::Error(error) => AnalyzeSection::Error(error.clone()),
    }
}

#[cfg(test)]
fn error_source<T>(err: crate::ClientError) -> SectionSource<T> {
    SectionSource::Error(structured_client_error(&err))
}

#[cfg(test)]
mod tests {
    use crate::ClientError;
    use crate::cli::AnalyzeSectionArg;
    use crate::cli::commands::fundamentals::FundamentalsRecord;
    use crate::cli::commands::industry::IndustryRsRecord;
    use crate::cli::commands::market_data::MarketDataRecord;
    use crate::cli::commands::ownership::OwnershipSummaryRecord;
    use crate::cli::commands::ratings::RatingsRecord;

    use super::{
        AnalyzeOutput, AnalyzeSection, SectionSelection, SectionSource, analyze_output,
        analyze_records, error_source, section_one_for_symbol, section_vec_for_symbol,
    };

    fn snapshot(symbol: &str) -> MarketDataRecord {
        MarketDataRecord {
            symbol: symbol.to_string(),
            company_name: Some("Example Corp".to_string()),
            instrument_type: None,
            ipo_date: None,
            comp_rating: Some(95),
            rs_rating: None,
            eps_rating: None,
            smr_rating: None,
            ad_rating: None,
            market_cap: None,
            avg_dollar_volume_50d: None,
            up_down_volume_ratio: None,
            short_interest_pct_float: None,
            short_interest_days_to_cover: None,
            industry_name: None,
            industry_sector: None,
            industry_stocks_in_group: None,
            funds_pct_float_held: None,
            eps_due_date: None,
            eps_due_date_status: None,
            debt_pct: None,
            rd_pct_last_qtr: None,
            decode_error: None,
        }
    }

    fn rating(symbol: &str, value: i64) -> RatingsRecord {
        RatingsRecord {
            symbol: symbol.to_string(),
            period: Some("DAILY".to_string()),
            period_offset: Some("CURRENT".to_string()),
            letter_value: Some("A".to_string()),
            rs_rating: Some(value),
            rs_line_new_high: Some(true),
        }
    }

    fn fundamental(symbol: &str, metric: &str) -> FundamentalsRecord {
        FundamentalsRecord {
            symbol: symbol.to_string(),
            company_name: None,
            metric: metric.to_string(),
            period_offset: Some("CURRENT".to_string()),
            period: None,
            value: Some("1.23".to_string()),
            pct_change_yoy: None,
            revision_direction: None,
        }
    }

    fn industry(symbol: &str, group_rank: i64) -> IndustryRsRecord {
        IndustryRsRecord {
            symbol: symbol.to_string(),
            group_rank: Some(group_rank),
        }
    }

    fn ownership(symbol: &str) -> OwnershipSummaryRecord {
        OwnershipSummaryRecord {
            symbol: symbol.to_string(),
            funds_float_pct_held: Some("12%".to_string()),
            date: Some("2026-03-31".to_string()),
            num_funds_held: Some("100".to_string()),
        }
    }

    #[test]
    fn section_selection_defaults_to_all_sections() {
        let selection = SectionSelection::from_sections(&[]);

        assert!(selection.snapshot);
        assert!(selection.ratings);
        assert!(selection.fundamentals);
        assert!(selection.industry);
        assert!(selection.ownership);
    }

    #[test]
    fn section_selection_respects_requested_subset() {
        let selection = SectionSelection::from_sections(&[
            AnalyzeSectionArg::Snapshot,
            AnalyzeSectionArg::Ratings,
        ]);

        assert!(selection.snapshot);
        assert!(selection.ratings);
        assert!(!selection.fundamentals);
        assert!(!selection.industry);
        assert!(!selection.ownership);
    }

    #[test]
    fn analyze_output_is_single_object_for_one_symbol() {
        let records = analyze_records(
            &["AAPL"],
            None,
            Some(&SectionSource::Data(vec![rating("AAPL", 93)])),
            None,
            Some(&SectionSource::Data(vec![industry("AAPL", 88)])),
            None,
        );

        assert!(matches!(analyze_output(records), AnalyzeOutput::Single(_)));
    }

    #[test]
    fn analyze_output_is_array_for_multiple_symbols() {
        let records = analyze_records(
            &["AAPL", "MSFT"],
            None,
            Some(&SectionSource::Data(vec![
                rating("AAPL", 93),
                rating("MSFT", 82),
            ])),
            None,
            None,
            None,
        );

        assert!(matches!(
            analyze_output(records),
            AnalyzeOutput::Multiple(_)
        ));
    }

    #[test]
    fn analyze_records_attach_data_to_each_symbol() {
        let records = analyze_records(
            &["AAPL", "MSFT"],
            Some(&SectionSource::Data(vec![
                snapshot("AAPL"),
                snapshot("MSFT"),
            ])),
            Some(&SectionSource::Data(vec![
                rating("AAPL", 93),
                rating("MSFT", 82),
            ])),
            Some(&SectionSource::Data(vec![
                fundamental("AAPL", "reported_eps"),
                fundamental("MSFT", "reported_sales"),
            ])),
            Some(&SectionSource::Data(vec![
                industry("AAPL", 88),
                industry("MSFT", 71),
            ])),
            Some(&SectionSource::Data(vec![
                ownership("AAPL"),
                ownership("MSFT"),
            ])),
        );

        assert_eq!(records.len(), 2);
        assert!(matches!(
            &records[0].snapshot,
            Some(AnalyzeSection::Data(row)) if row.comp_rating == Some(95)
        ));
        assert!(
            matches!(&records[0].ratings, Some(AnalyzeSection::Data(rows)) if rows[0].rs_rating == Some(93))
        );
        assert!(matches!(
            &records[0].fundamentals,
            Some(AnalyzeSection::Data(rows)) if rows[0].metric == "reported_eps"
        ));
        assert!(
            matches!(&records[1].industry, Some(AnalyzeSection::Data(row)) if row.group_rank == Some(71))
        );
        assert!(matches!(
            &records[1].ownership,
            Some(AnalyzeSection::Data(rows)) if rows[0].num_funds_held.as_deref() == Some("100")
        ));
    }

    #[test]
    fn section_one_embeds_client_error_for_failed_section() {
        let source = error_source::<MarketDataRecord>(ClientError::Status {
            status: 500,
            body: "upstream failed".to_string(),
        });
        let section = section_one_for_symbol(&source, "AAPL", |record| &record.symbol, "missing");

        assert!(matches!(section, AnalyzeSection::Error(error) if error.kind == "api_error"));
    }

    #[test]
    fn section_vec_reports_no_results_for_missing_symbol_data() {
        let section = section_vec_for_symbol(
            &SectionSource::Data(vec![rating("AAPL", 93)]),
            "MSFT",
            |record| &record.symbol,
            "missing rating data",
        );

        assert!(matches!(section, AnalyzeSection::Error(error) if error.kind == "no_results"));
    }

    #[test]
    fn section_vec_embeds_client_error_for_failed_section() {
        let source = error_source::<RatingsRecord>(ClientError::Status {
            status: 429,
            body: "rate limited".to_string(),
        });
        let section = section_vec_for_symbol(&source, "AAPL", |record| &record.symbol, "missing");

        assert!(matches!(section, AnalyzeSection::Error(error) if error.kind == "rate_limit"));
    }
}
