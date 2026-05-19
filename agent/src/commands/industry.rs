//! Industry group data commands.

use clap::Subcommand;
use serde::Serialize;
use tracing::instrument;

use marketsurge_client::industry::{IndustryGroupRsItem, IndustryOverviewItem};

use crate::cli::{IndustryArgs, SymbolsArgs};
use crate::common::auth::handle_api_error;
use crate::common::command::{run_command, zip_symbols};

/// Industry subcommands.
#[derive(Debug, Subcommand)]
pub enum IndustryCommand {
    /// Fetch industry group relative strength rating.
    Rs(SymbolsArgs),
    /// Fetch industry overview with rankings and sector data.
    Overview(SymbolsArgs),
}

/// Flat output record for industry group relative strength.
#[derive(Debug, Clone, Serialize)]
pub struct IndustryRsRecord {
    /// Ticker symbol.
    pub symbol: String,
    /// Industry group RS value (6-month current).
    pub group_rs: Option<i64>,
}

/// Flat output record for industry overview data.
#[derive(Debug, Clone, Serialize)]
pub struct IndustryOverviewRecord {
    /// Symbol identifier.
    pub symbol: String,
    /// Industry group name.
    pub name: Option<String>,
    /// Sector name.
    pub sector: Option<String>,
    /// Numeric industry code.
    pub ind_code: Option<i64>,
    /// Group market value in billions (formatted).
    pub group_market_value_billions: Option<String>,
    /// Number of stocks at new highs in the group.
    pub num_new_highs: Option<i64>,
    /// Number of stocks at new lows in the group.
    pub num_new_lows: Option<i64>,
    /// Total number of stocks in the group.
    pub num_stocks: Option<i64>,
    /// Current group rank.
    pub group_rank: Option<i64>,
    /// Price percent change vs 1 day ago (formatted).
    pub pct_change_1d: Option<String>,
    /// Price percent change year-to-date (formatted).
    pub pct_change_ytd: Option<String>,
    /// EPS rank within industry group.
    pub eps_rank: Option<i64>,
    /// RS rank within industry group.
    pub rs_rank: Option<i64>,
    /// Accumulation/Distribution rank within industry group.
    pub ad_rank: Option<i64>,
    /// SMR rank within industry group.
    pub smr_rank: Option<i64>,
    /// Composite rank within industry group.
    pub comp_rank: Option<i64>,
}

/// Handles the industry command group.
#[instrument(skip_all)]
pub async fn handle(args: &IndustryArgs, json_table: bool) -> i32 {
    match &args.command {
        IndustryCommand::Rs(a) => execute_rs(a, json_table).await,
        IndustryCommand::Overview(a) => execute_overview(a, json_table).await,
    }
}

/// Transforms raw industry group RS response items into flat output records.
fn flatten_industry_rs(
    symbols: &[&str],
    market_data: &[IndustryGroupRsItem],
) -> Vec<IndustryRsRecord> {
    zip_symbols(symbols, market_data)
        .map(|(symbol, item)| {
            let group_rs = item
                .industry
                .as_ref()
                .and_then(|ind| ind.group_rs.first())
                .and_then(|v| v.value);

            IndustryRsRecord {
                symbol: symbol.to_string(),
                group_rs,
            }
        })
        .collect()
}

#[instrument(skip_all)]
async fn execute_rs(args: &SymbolsArgs, json_table: bool) -> i32 {
    run_command(
        &args.symbols,
        json_table,
        |client, symbol_refs| async move {
            let response = client
                .industry_group_rs(&symbol_refs, None)
                .await
                .map_err(handle_api_error)?;

            Ok(flatten_industry_rs(&symbol_refs, &response.market_data))
        },
    )
    .await
}

/// Transforms raw industry overview response items into flat output records.
fn flatten_industry_overview(market_data: &[IndustryOverviewItem]) -> Vec<IndustryOverviewRecord> {
    market_data
        .iter()
        .map(|item| {
            let symbol = item.id.clone().unwrap_or_default();
            let industry = item.industry.as_ref();
            let ratings = item.ratings.as_ref();
            let rank = ratings.and_then(|r| r.industry.as_ref());

            let group_rank = industry
                .map(|ind| ind.group_ranks.as_slice())
                .unwrap_or_default()
                .first()
                .and_then(|r| r.value);

            let pct_change_1d = industry
                .map(|ind| ind.price_percent_change_vs.as_slice())
                .unwrap_or_default()
                .iter()
                .find(|v| v.subject.as_deref() == Some("VS_1D_AGO"))
                .and_then(|v| v.formatted_value.clone());

            let pct_change_ytd = industry
                .map(|ind| ind.price_percent_change_vs.as_slice())
                .unwrap_or_default()
                .iter()
                .find(|v| v.subject.as_deref() == Some("VS_YTD"))
                .and_then(|v| v.formatted_value.clone());

            IndustryOverviewRecord {
                symbol,
                name: industry.and_then(|i| i.name.clone()),
                sector: industry.and_then(|i| i.sector.clone()),
                ind_code: industry.and_then(|i| i.ind_code),
                group_market_value_billions: industry
                    .and_then(|i| i.group_market_value_in_billions.as_ref())
                    .and_then(|v| v.formatted_value.clone()),
                num_new_highs: industry.and_then(|i| i.num_new_highs_in_group),
                num_new_lows: industry.and_then(|i| i.num_new_lows_in_group),
                num_stocks: industry.and_then(|i| i.number_of_stocks_in_group),
                group_rank,
                pct_change_1d,
                pct_change_ytd,
                eps_rank: rank.and_then(|r| r.eps_rank_in_industry_group),
                rs_rank: rank.and_then(|r| r.rs_rank_in_industry_group),
                ad_rank: rank.and_then(|r| r.ad_rank_in_industry_group),
                smr_rank: rank.and_then(|r| r.smr_rank_in_industry_group),
                comp_rank: rank.and_then(|r| r.comp_rank_in_industry_group),
            }
        })
        .collect()
}

#[instrument(skip_all)]
async fn execute_overview(args: &SymbolsArgs, json_table: bool) -> i32 {
    run_command(
        &args.symbols,
        json_table,
        |client, symbol_refs| async move {
            let response = client
                .industry_overview(&symbol_refs, None)
                .await
                .map_err(handle_api_error)?;

            Ok(flatten_industry_overview(&response.market_data))
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use marketsurge_client::industry::{
        IndustryGroupRsIndustry, IndustryGroupRsValue, IndustryOverviewIndustry,
        IndustryOverviewRatings, IndustryRankInGroup,
    };
    use marketsurge_client::market_data::{MdGroupRank, MdPercentChangeVs};

    // -----------------------------------------------------------------------
    // flatten_industry_rs
    // -----------------------------------------------------------------------

    #[test]
    fn flatten_industry_rs_happy_path() {
        let items = vec![
            IndustryGroupRsItem {
                origin_request: None,
                industry: Some(IndustryGroupRsIndustry {
                    group_rs: vec![IndustryGroupRsValue { value: Some(85) }],
                }),
            },
            IndustryGroupRsItem {
                origin_request: None,
                industry: Some(IndustryGroupRsIndustry {
                    group_rs: vec![IndustryGroupRsValue { value: Some(42) }],
                }),
            },
        ];
        let symbols = ["AAPL", "MSFT"];

        let records = flatten_industry_rs(&symbols, &items);

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].symbol, "AAPL");
        assert_eq!(records[0].group_rs, Some(85));
        assert_eq!(records[1].symbol, "MSFT");
        assert_eq!(records[1].group_rs, Some(42));
    }

    #[test]
    fn flatten_industry_rs_empty_market_data() {
        let records = flatten_industry_rs(&["AAPL"], &[]);
        assert!(records.is_empty());
    }

    #[test]
    fn flatten_industry_rs_none_industry() {
        let items = vec![IndustryGroupRsItem {
            origin_request: None,
            industry: None,
        }];

        let records = flatten_industry_rs(&["AAPL"], &items);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].symbol, "AAPL");
        assert_eq!(records[0].group_rs, None);
    }

    // -----------------------------------------------------------------------
    // flatten_industry_overview
    // -----------------------------------------------------------------------

    #[test]
    fn flatten_industry_overview_happy_path() {
        let items = vec![IndustryOverviewItem {
            id: Some("13-4698".to_string()),
            industry: Some(IndustryOverviewIndustry {
                name: Some("Elec-Semicondctor Fablss".to_string()),
                ind_code: Some(7010),
                news_code: Some("I/SEMF".to_string()),
                sector: Some("CHIPS".to_string()),
                group_market_value_in_billions: None,
                num_new_highs_in_group: Some(1),
                num_new_lows_in_group: Some(0),
                number_of_stocks_in_group: Some(45),
                group_ranks: vec![MdGroupRank {
                    value: Some(16),
                    period: Some("P6M".to_string()),
                    period_offset: Some("CURRENT".to_string()),
                }],
                price_percent_change_vs: vec![
                    MdPercentChangeVs {
                        value: None,
                        formatted_value: Some("-1.22%".to_string()),
                        subject: Some("VS_1D_AGO".to_string()),
                        period: None,
                    },
                    MdPercentChangeVs {
                        value: None,
                        formatted_value: Some("27.07%".to_string()),
                        subject: Some("VS_YTD".to_string()),
                        period: None,
                    },
                ],
            }),
            ratings: Some(IndustryOverviewRatings {
                has_ratings_data: Some(true),
                industry: Some(IndustryRankInGroup {
                    ad_rank_in_industry_group: Some(10),
                    comp_rank_in_industry_group: Some(1),
                    eps_rank_in_industry_group: Some(4),
                    number_of_stocks_in_group: Some(45),
                    rs_rank_in_industry_group: Some(2),
                    smr_rank_in_industry_group: Some(8),
                }),
            }),
        }];

        let records = flatten_industry_overview(&items);

        assert_eq!(records.len(), 1);
        let r = &records[0];
        assert_eq!(r.symbol, "13-4698");
        assert_eq!(r.name.as_deref(), Some("Elec-Semicondctor Fablss"));
        assert_eq!(r.sector.as_deref(), Some("CHIPS"));
        assert_eq!(r.ind_code, Some(7010));
        assert_eq!(r.num_stocks, Some(45));
        assert_eq!(r.group_rank, Some(16));
        assert_eq!(r.pct_change_1d.as_deref(), Some("-1.22%"));
        assert_eq!(r.pct_change_ytd.as_deref(), Some("27.07%"));
        assert_eq!(r.eps_rank, Some(4));
        assert_eq!(r.rs_rank, Some(2));
        assert_eq!(r.comp_rank, Some(1));
    }

    #[test]
    fn flatten_industry_overview_empty_market_data() {
        let records = flatten_industry_overview(&[]);
        assert!(records.is_empty());
    }

    #[test]
    fn flatten_industry_overview_none_fields() {
        let items = vec![IndustryOverviewItem {
            id: None,
            industry: None,
            ratings: None,
        }];

        let records = flatten_industry_overview(&items);

        assert_eq!(records.len(), 1);
        let r = &records[0];
        assert_eq!(r.symbol, "");
        assert!(r.name.is_none());
        assert!(r.sector.is_none());
        assert!(r.group_rank.is_none());
        assert!(r.pct_change_1d.is_none());
        assert!(r.pct_change_ytd.is_none());
        assert!(r.eps_rank.is_none());
    }

    #[test]
    fn flatten_industry_overview_subject_filter() {
        // Only VS_YTD present, no VS_1D_AGO - verifies the .find() filter
        let items = vec![IndustryOverviewItem {
            id: Some("TEST".to_string()),
            industry: Some(IndustryOverviewIndustry {
                name: None,
                ind_code: None,
                news_code: None,
                sector: None,
                group_market_value_in_billions: None,
                num_new_highs_in_group: None,
                num_new_lows_in_group: None,
                number_of_stocks_in_group: None,
                group_ranks: vec![],
                price_percent_change_vs: vec![MdPercentChangeVs {
                    value: None,
                    formatted_value: Some("15.00%".to_string()),
                    subject: Some("VS_YTD".to_string()),
                    period: None,
                }],
            }),
            ratings: None,
        }];

        let records = flatten_industry_overview(&items);

        assert_eq!(records.len(), 1);
        assert!(records[0].pct_change_1d.is_none());
        assert_eq!(records[0].pct_change_ytd.as_deref(), Some("15.00%"));
    }
}
