use common::rlp::Nullable;
use cosmwasm_std::{Coin, Deps, DepsMut, Env, MessageInfo, Response, Storage, Uint128};
use cw_xcall_lib::network_address::NetId;
use router_api::Message;

use crate::{
    error::ContractError,
    state::{CwIbcConnection, IbcConfig},
    types::LOG_PREFIX,
};

impl<'a> CwIbcConnection<'a> {
    pub fn send_message(
        &self,
        deps: Deps,
        nid: NetId,
        messages: Message,
    ) -> Result<Response, ContractError> {
        println!("{LOG_PREFIX} Packet Validated");
        let ibc_config = self.get_ibc_config(deps.storage, &nid)?;

        let sequence_number_host = self.query_host_sequence_no(deps, &ibc_config)?;

        let timeout_height =
            self.query_timeout_height(deps, &ibc_config.src_endpoint().channel_id)?;

        let packet_data =
            self.create_packet(ibc_config, timeout_height, sequence_number_host, messages);

        println!("{} Raw Packet Created {:?}", LOG_PREFIX, &packet_data);

        let submessage = self.call_host_send_message(deps, packet_data)?;
        Ok(Response::new()
            .add_submessage(submessage)
            .add_attribute("method", "send_message"))
    }

    fn write_acknowledgement(
        &self,
        store: &mut dyn Storage,
        config: &IbcConfig,
        msg: Vec<u8>,
        sn: i64,
    ) -> Result<Response, ContractError> {
        let channel_id = config.src_endpoint().channel_id.clone();
        let packet = self.get_incoming_packet(store, &channel_id, sn)?;
        self.remove_incoming_packet(store, &channel_id, sn);
        let submsg = self.call_host_write_acknowledgement(store, packet, msg)?;
        Ok(Response::new().add_submessage(submsg))
    }
}

fn get_amount_for_denom(funds: &Vec<Coin>, target_denom: String) -> Uint128 {
    for coin in funds.iter() {
        if coin.denom == target_denom {
            return coin.amount;
        }
    }
    Uint128::zero()
}
#[cfg(feature = "native_ibc")]
impl<'a> CwIbcConnection<'a> {
    /// This function creates an IBC message to send a packet with a timeout to a destination endpoint.
    ///
    /// Arguments:
    ///
    /// * `deps`: `deps` is a mutable reference to the dependencies of the contract. It is used to
    /// interact with the storage and other modules of the contract.
    /// * `env`: `env` is an object that contains information about the current blockchain environment,
    /// such as the current block height, time, and chain ID. It is used to calculate the timeout for the
    /// IBC packet.
    /// * `time_out_height`: The height of the block at which the timeout for the packet will occur.
    /// * `message`: `message` is a `CallServiceMessage` struct that contains the information needed to
    /// create a request packet to be sent over the IBC channel. This includes the method name, input
    /// arguments, and any other relevant data needed for the service call.
    ///
    /// Returns:
    ///
    /// a `Result` with an `IbcMsg` on success or a `ContractError` on failure.
    fn create_request_packet(
        &self,
        deps: DepsMut,
        env: Env,
        time_out_height: u64,
        message: Message,
    ) -> Result<IbcMsg, ContractError> {
        let ibc_config = self
            .ibc_config()
            .load(deps.as_ref().storage)
            .map_err(ContractError::Std)?;

        let timeout_block = IbcTimeoutBlock {
            revision: 0,
            height: time_out_height,
        };
        let timeout = IbcTimeout::with_both(timeout_block, env.block.time.plus_seconds(300));

        Ok(IbcMsg::SendPacket {
            channel_id: ibc_config.dst_endpoint().channel_id.clone(),
            data: to_binary(&message).unwrap(),
            timeout,
        })
    }
}
