use super::*;

/// This is a Rust struct representing a message to instantiate a contract with timeout height and IBC
/// host address.
///
/// Properties:
///
/// * `timeout_height`: `timeout_height` is a field of type `u64` (unsigned 64-bit integer) in the
/// `InstantiateMsg` struct. It represents the block height at which the transaction will timeout if it
/// has not been included in a block by that height. This is used to prevent transactions from being
/// * `ibc_host`: `ibc_host` is a field of type `Addr` in the `InstantiateMsg` struct. It likely
/// represents the address of the IBC host that the message is being sent to. However, without more
/// context it's difficult to say for sure.
#[cw_serde]
pub struct InstantiateMsg {
    pub ibc_host: Addr,
    pub port_id: String,
    pub xcall_address: Addr,
    pub router_address: Addr,
    pub denom: String,
}

use cw_common::cw_types::{
    CwChannelCloseMsg, CwChannelConnectMsg, CwChannelOpenMsg, CwPacketAckMsg, CwPacketReceiveMsg,
    CwPacketTimeoutMsg,
};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cw_xcall_lib::network_address::NetId;
use router_api::Message as RouteMessage;

#[cw_serde]
pub enum ExecuteMsg {
    SetAdmin {
        address: String,
    },

    ConfigureConnection {
        connection_id: String,
        counterparty_port_id: String,
        counterparty_nid: NetId,
        client_id: String,
        timeout_height: u64,
    },
    OverrideConnection {
        connection_id: String,
        counterparty_port_id: String,
        counterparty_nid: NetId,
        client_id: String,
        timeout_height: u64,
    },
    RouteMessages(Vec<RouteMessage>),

    #[cfg(not(feature = "native_ibc"))]
    IbcChannelOpen {
        msg: CwChannelOpenMsg,
    },

    #[cfg(not(feature = "native_ibc"))]
    IbcChannelConnect {
        msg: CwChannelConnectMsg,
    },
    #[cfg(not(feature = "native_ibc"))]
    IbcChannelClose {
        msg: CwChannelCloseMsg,
    },
    #[cfg(not(feature = "native_ibc"))]
    IbcPacketReceive {
        msg: CwPacketReceiveMsg,
    },
    #[cfg(not(feature = "native_ibc"))]
    IbcPacketAck {
        msg: CwPacketAckMsg,
    },
    #[cfg(not(feature = "native_ibc"))]
    IbcPacketTimeout {
        msg: CwPacketTimeoutMsg,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub channel_id: String,
    pub port: String,
    pub destination_channel_id: String,
    pub destination_port_id: String,
    pub light_client_id: String,
    pub timeout_height: u64,
}

#[cw_serde]
#[derive(QueryResponses)]
/// This is a Rust enum representing different types of queries that can be made to the contract. Each
/// variant of the enum corresponds to a specific query and has a return type specified using the
/// `#[returns]` attribute.
pub enum QueryMsg {
    #[returns(String)]
    GetAdmin {},
    #[returns(u64)]
    GetTimeoutHeight { channel_id: String },
    #[returns(ConfigResponse)]
    GetIbcConfig { nid: NetId },
}
