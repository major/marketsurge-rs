//! Industry group data commands.

use clap::Subcommand;
use serde::Serialize;
use tracing::instrument;

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

            let mut records = Vec::new();

            for (symbol, item) in zip_symbols(&symbol_refs, &response.market_data) {
                let group_rs = item
                    .industry
                    .as_ref()
                    .and_then(|ind| ind.group_rs.first())
                    .and_then(|v| v.value);

                records.push(IndustryRsRecord {
                    symbol: symbol.to_string(),
                    group_rs,
                });
            }

            Ok(records)
        },
    )
    .await
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

            let mut records = Vec::new();

            for item in &response.market_data {
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

                records.push(IndustryOverviewRecord {
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
                });
            }

            Ok(records)
        },
    )
    .await
}
