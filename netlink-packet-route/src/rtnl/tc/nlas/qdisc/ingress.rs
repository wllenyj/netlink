// SPDX-License-Identifier: MIT

/// Ingress qdisc
/// 
/// This effectively allows you to police incoming traffic, before it even enters
/// the IP stack.
/// The ingress qdisc itself does not require any parameters. It differs from
/// other qdiscs in that it does not occupy the root of a device. 

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Ingress();

impl Ingress {
    pub const KIND: &'static str = "ingress";
}
