pub mod execute {
    use crate::axelar::events::GatewayEvent;
    use crate::state::CwIbcConnection;
    use crate::ContractError;
    use axelar_wasm_std::{FnExt, VerificationStatus};
    use cosmwasm_std::{DepsMut, Event, Response, Storage, WasmMsg};
    use error_stack::ResultExt;
    use itertools::Itertools;
    use router_api::{client::Router, Message};

    // pub fn verify_messages(
    //     verifier: &aggregate_verifier::Client,
    //     msgs: Vec<Message>,
    // ) -> Result<Response, ContractError> {
    //     apply(verifier, msgs, |msgs_by_status| {
    //         verify(verifier, msgs_by_status)
    //     })
    // }

    pub(crate) fn route_incoming_messages(
        router: &Router,
        msgs: Vec<Message>,
    ) -> Result<Response, ContractError> {
        check_for_duplicates(msgs)?.then(|msgs| {
            let mut vec: Vec<(VerificationStatus, Vec<Message>)> = Vec::new();
            vec.push((VerificationStatus::SucceededOnChain, msgs));
            return route(router, vec)
                .then(|(msgs, events)| Response::new().add_messages(msgs).add_events(events))
                .then(Ok);
        })
    }

    // because the messages came from the router, we can assume they are already verified
    pub(crate) fn route_outgoing_messages(
        deps: DepsMut,
        verified: Vec<Message>,
    ) -> Result<Response, ContractError> {
        let msgs = check_for_duplicates(verified)?;
        let connection = CwIbcConnection::new();
        for msg in msgs.iter() {
            let nid = connection.get_counterparty_nid_by_chainid(deps.storage, &msg.cc_id)?;
            connection.save_outgoing_msg(deps.storage, msg.cc_id.clone(), msg)?;
            connection.send_message(deps.as_ref(), nid, msg.clone())?;
        }

        Ok(Response::new().add_events(
            msgs.into_iter()
                .map(|msg| GatewayEvent::Routing { msg }.into()),
        ))
    }

    fn check_for_duplicates(msgs: Vec<Message>) -> Result<Vec<Message>, ContractError> {
        let duplicates: Vec<_> = msgs
            .iter()
            // the following two map instructions are separated on purpose
            // so the duplicate check is done on the typed id instead of just a string
            .map(|m| &m.cc_id)
            .duplicates()
            .map(|cc_id| cc_id.to_string())
            .collect();
        if !duplicates.is_empty() {
            return Err(ContractError::DuplicateMessageIds);
        }
        Ok(msgs)
    }

    // fn group_by_status(
    //     msgs_with_status: impl Iterator<Item = (Message, VerificationStatus)>,
    // ) -> Vec<(VerificationStatus, Vec<Message>)> {
    //     msgs_with_status
    //         .map(|(msg, status)| (status, msg))
    //         .into_group_map()
    //         .into_iter()
    //         // sort by verification status so the order of messages is deterministic
    //         .sorted_by_key(|(status, _)| *status)
    //         .collect()
    // }

    // fn verify(
    //     verifier: &aggregate_verifier::Client,
    //     msgs_by_status: Vec<(VerificationStatus, Vec<Message>)>,
    // ) -> (Option<WasmMsg>, Vec<Event>) {
    //     msgs_by_status
    //         .into_iter()
    //         .map(|(status, msgs)| {
    //             (
    //                 filter_verifiable_messages(status, &msgs),
    //                 into_verify_events(status, msgs),
    //             )
    //         })
    //         .then(flat_unzip)
    //         .then(|(msgs, events)| (verifier.verify_messages(msgs), events))
    // }

    fn route(
        router: &Router,
        msgs_by_status: Vec<(VerificationStatus, Vec<Message>)>,
    ) -> (Option<WasmMsg>, Vec<Event>) {
        msgs_by_status
            .into_iter()
            .map(|(status, msgs)| {
                (
                    filter_routable_messages(status, &msgs),
                    into_route_events(status, msgs),
                )
            })
            .then(flat_unzip)
            .then(|(msgs, events)| (router.route(msgs), events))
    }

    // not all messages are verifiable, so it's better to only take a reference and allocate a vector on demand
    // instead of requiring the caller to allocate a vector for every message
    fn filter_verifiable_messages(status: VerificationStatus, msgs: &[Message]) -> Vec<Message> {
        match status {
            VerificationStatus::None
            | VerificationStatus::NotFound
            | VerificationStatus::FailedToVerify => msgs.to_vec(),
            _ => vec![],
        }
    }

    fn into_verify_events(status: VerificationStatus, msgs: Vec<Message>) -> Vec<Event> {
        match status {
            VerificationStatus::None
            | VerificationStatus::NotFound
            | VerificationStatus::FailedToVerify
            | VerificationStatus::InProgress => {
                messages_into_events(msgs, |msg| GatewayEvent::Verifying { msg })
            }
            VerificationStatus::SucceededOnChain => {
                messages_into_events(msgs, |msg| GatewayEvent::AlreadyVerified { msg })
            }
            VerificationStatus::FailedOnChain => {
                messages_into_events(msgs, |msg| GatewayEvent::AlreadyRejected { msg })
            }
        }
    }

    // not all messages are routable, so it's better to only take a reference and allocate a vector on demand
    // instead of requiring the caller to allocate a vector for every message
    fn filter_routable_messages(status: VerificationStatus, msgs: &[Message]) -> Vec<Message> {
        if status == VerificationStatus::SucceededOnChain {
            msgs.to_vec()
        } else {
            vec![]
        }
    }

    fn into_route_events(status: VerificationStatus, msgs: Vec<Message>) -> Vec<Event> {
        match status {
            VerificationStatus::SucceededOnChain => {
                messages_into_events(msgs, |msg| GatewayEvent::Routing { msg })
            }
            _ => messages_into_events(msgs, |msg| GatewayEvent::UnfitForRouting { msg }),
        }
    }

    fn flat_unzip<A, B>(x: impl Iterator<Item = (Vec<A>, Vec<B>)>) -> (Vec<A>, Vec<B>) {
        let (x, y): (Vec<_>, Vec<_>) = x.unzip();
        (
            x.into_iter().flatten().collect(),
            y.into_iter().flatten().collect(),
        )
    }

    fn messages_into_events(
        msgs: Vec<Message>,
        transform: fn(Message) -> GatewayEvent,
    ) -> Vec<Event> {
        msgs.into_iter().map(|msg| transform(msg).into()).collect()
    }
}

pub mod query {}

pub mod events {
    use cosmwasm_std::{Attribute, Event};
    use router_api::Message;

    pub enum GatewayEvent {
        Verifying { msg: Message },
        AlreadyVerified { msg: Message },
        AlreadyRejected { msg: Message },
        Routing { msg: Message },
        UnfitForRouting { msg: Message },
    }

    fn make_message_event(event_name: &str, msg: Message) -> Event {
        let attrs: Vec<Attribute> = msg.into();

        Event::new(event_name).add_attributes(attrs)
    }

    impl From<GatewayEvent> for Event {
        fn from(other: GatewayEvent) -> Self {
            match other {
                GatewayEvent::Verifying { msg } => make_message_event("verifying", msg),
                GatewayEvent::AlreadyVerified { msg } => {
                    make_message_event("already_verified", msg)
                }
                GatewayEvent::AlreadyRejected { msg } => {
                    make_message_event("already_rejected", msg)
                }
                GatewayEvent::Routing { msg } => make_message_event("routing", msg),
                GatewayEvent::UnfitForRouting { msg } => {
                    make_message_event("unfit_for_routing", msg)
                }
            }
        }
    }
}
