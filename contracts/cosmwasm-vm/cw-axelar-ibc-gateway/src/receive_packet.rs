use std::str::from_utf8;

use self::types::message::Message;

use super::*;

use common::rlp;
use cosmwasm_std::{coins, BankMsg, DepsMut};
use cw_common::{cw_println, from_binary_response};
use router_api::{client::Router, Message as RouteMessages};

impl<'a> CwIbcConnection<'a> {
    /// This function receives packet data, decodes it, and then handles either a request or a response
    /// based on the message type.
    ///
    /// Arguments:
    ///
    /// * `deps`: `deps` is a `DepsMut` object, which is short for "dependencies mutable". It is a
    /// struct that provides access to the dependencies needed by the contract to execute its logic.
    /// These dependencies include the storage, the API to interact with the blockchain, and the querier
    /// to query data
    /// * `message`: The `message` parameter is of type `IbcPacket` and represents the packet received
    /// by the contract from another chain. It contains the data sent by the sender chain and metadata
    /// about the packet, such as the sender and receiver addresses, the sequence number, and the
    /// timeout height.
    ///
    /// Returns:
    ///
    /// a `Result` object with either an `IbcReceiveResponse` or a `ContractError`.
    pub fn do_packet_receive(
        &self,
        deps: DepsMut,
        packet: CwPacket,
        relayer: Addr,
    ) -> Result<CwReceiveResponse, ContractError> {
        let route_message: RouteMessages = from_binary_response(&packet.data.0).unwrap();
        let router_address = self.router().load(deps.storage)?;
        let router = Router {
            address: router_address,
        };
        let msgs = vec![route_message];
        axelar::execute::route_incoming_messages(&router, msgs)?;
        Ok(CwReceiveResponse::new())
    }
}
