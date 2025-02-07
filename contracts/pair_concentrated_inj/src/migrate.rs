use astroport::asset::PairInfo;
use astroport::factory::PairType;
use cosmwasm_std::{attr, entry_point, DepsMut, Env, Response, StdError, StdResult};
use cw2::{set_contract_version, CONTRACT};
use cw_storage_plus::Item;
use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQueryWrapper};

use crate::contract::{CONTRACT_NAME, CONTRACT_VERSION};
use crate::orderbook::state::OrderbookState;
use astroport::pair_concentrated_inj::MigrateMsg;
use astroport_pair_concentrated::state::Config as CLConfig;

use crate::state::{AmpGamma, Config, PoolParams, PoolState, PriceState, CONFIG};

const MIGRATE_FROM: &str = "astroport-pair-concentrated";
const MIGRATION_VERSION: &str = "2.0.5";

/// Manages the contract migration.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    msg: MigrateMsg,
) -> StdResult<Response<InjectiveMsgWrapper>> {
    let mut attrs = vec![];

    let contract_info = CONTRACT.load(deps.storage)?;
    match msg {
        MigrateMsg::MigrateToOrderbook { params } => {
            if contract_info.contract != MIGRATE_FROM || contract_info.version != MIGRATION_VERSION
            {
                return Err(StdError::generic_err(format!(
                    "Can't migrate from {} {}",
                    contract_info.contract, contract_info.version
                )));
            }

            let config: CLConfig = Item::new("config").load(deps.storage)?;
            let base_precision =
                config.pair_info.asset_infos[0].decimals(&deps.querier, &config.factory_addr)?;
            let ob_state = OrderbookState::new(
                deps.querier,
                &env,
                &params.market_id,
                params.orders_number,
                params.min_trades_to_avg,
                &config.pair_info.asset_infos,
                base_precision,
            )?;
            CONFIG.save(deps.storage, &config.into())?;
            ob_state.save(deps.storage)?;

            attrs.push(attr("action", "migrate_to_orderbook"));
            attrs.push(attr("subaccount_id", ob_state.subaccount.to_string()))
        }
        MigrateMsg::Migrate {} => {
            let contract_info = cw2::get_contract_version(deps.storage)?;
            match contract_info.contract.as_str() {
                CONTRACT_NAME => match contract_info.version.as_str() {
                    "2.0.3" | "2.0.4" => {}
                    _ => {
                        return Err(StdError::generic_err(format!(
                            "Can't migrate from {} {}",
                            contract_info.contract, contract_info.version
                        )));
                    }
                },
                _ => {
                    return Err(StdError::generic_err(format!(
                        "Can't migrate from {} {}",
                        contract_info.contract, contract_info.version
                    )));
                }
            }
        }
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    attrs.extend([
        attr("previous_contract_name", contract_info.contract),
        attr("previous_contract_version", contract_info.version),
        attr("new_contract_name", CONTRACT_NAME),
        attr("new_contract_version", CONTRACT_VERSION),
    ]);
    Ok(Response::default().add_attributes(attrs))
}

impl From<CLConfig> for Config {
    fn from(val: CLConfig) -> Config {
        Config {
            pair_info: PairInfo {
                pair_type: PairType::Custom("concentrated_inj_orderbook".to_string()),
                ..val.pair_info
            },
            factory_addr: val.factory_addr,
            pool_params: PoolParams {
                mid_fee: val.pool_params.mid_fee,
                out_fee: val.pool_params.out_fee,
                fee_gamma: val.pool_params.fee_gamma,
                repeg_profit_threshold: val.pool_params.repeg_profit_threshold,
                min_price_scale_delta: val.pool_params.min_price_scale_delta,
                ma_half_time: val.pool_params.ma_half_time,
            },
            pool_state: PoolState {
                initial: AmpGamma {
                    amp: val.pool_state.initial.amp,
                    gamma: val.pool_state.initial.gamma,
                },
                future: AmpGamma {
                    amp: val.pool_state.future.amp,
                    gamma: val.pool_state.future.gamma,
                },
                future_time: val.pool_state.future_time,
                initial_time: val.pool_state.initial_time,
                price_state: PriceState {
                    oracle_price: val.pool_state.price_state.oracle_price,
                    last_price: val.pool_state.price_state.last_price,
                    price_scale: val.pool_state.price_state.price_scale,
                    last_price_update: val.pool_state.price_state.last_price_update,
                    xcp_profit: val.pool_state.price_state.xcp_profit,
                    xcp_profit_real: val.pool_state.price_state.xcp_profit_real,
                },
            },
            owner: val.owner,
        }
    }
}
