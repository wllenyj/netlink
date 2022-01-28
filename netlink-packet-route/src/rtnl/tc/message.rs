// SPDX-License-Identifier: MIT
use std::convert::From;

use anyhow::Context;

use crate::{
    constants::*,
    nlas::{
        self,
        tc::{Nla, Stats, Stats2, StatsBuffer},
        DefaultNla, NlaBuffer, NlasIterator,
    },
    parsers::{parse_string, parse_u8},
    traits::{Emitable, Parseable, ParseableParametrized},
    DecodeError, TcMessageBuffer, TC_HEADER_LEN,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TcMessage<A> {
    pub header: TcHeader,
    pub nlas: Vec<Nla<A>>,
}

impl<A> TcMessage<A> {
    pub fn into_parts(self) -> (TcHeader, Vec<Nla<A>>) {
        (self.header, self.nlas)
    }

    pub fn from_parts(header: TcHeader, nlas: Vec<Nla<A>>) -> Self {
        TcMessage { header, nlas }
    }
}

impl<A> Default for TcMessage<A> {
    fn default() -> Self {
        Self {
            nlas: Vec::new(),
            ..Default::default()
        }
    }
}

impl<A> From<i32> for TcMessage<A> {
    fn from(index: i32) -> Self {
        TcMessage {
            header: TcHeader {
                index,
                ..Default::default()
            },
            nlas: Vec::new(),
        }
    }
}

impl<A> From<u32> for TcMessage<A> {
    fn from(index: u32) -> Self {
        TcMessage {
            header: TcHeader {
                index: index as i32,
                ..Default::default()
            },
            nlas: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct TcHeader {
    pub family: u8,
    // Interface index
    pub index: i32,
    // Qdisc handle
    pub handle: u32,
    // Parent Qdisc
    pub parent: u32,
    pub info: u32,
}

impl Emitable for TcHeader {
    fn buffer_len(&self) -> usize {
        TC_HEADER_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let mut packet = TcMessageBuffer::new(buffer);
        packet.set_family(self.family);
        packet.set_index(self.index);
        packet.set_handle(self.handle);
        packet.set_parent(self.parent);
        packet.set_info(self.info);
    }
}

impl<A: nlas::Nla> Emitable for TcMessage<A> {
    fn buffer_len(&self) -> usize {
        self.header.buffer_len() + self.nlas.as_slice().buffer_len()
    }

    fn emit(&self, buffer: &mut [u8]) {
        self.header.emit(buffer);
        self.nlas
            .as_slice()
            .emit(&mut buffer[self.header.buffer_len()..]);
    }
}

impl<T: AsRef<[u8]>> Parseable<TcMessageBuffer<T>> for TcHeader {
    fn parse(buf: &TcMessageBuffer<T>) -> Result<Self, DecodeError> {
        Ok(Self {
            family: buf.family(),
            index: buf.index(),
            handle: buf.handle(),
            parent: buf.parent(),
            info: buf.info(),
        })
    }
}

impl<'a, T, A> Parseable<TcMessageBuffer<&'a T>> for TcMessage<A>
where
    T: AsRef<[u8]>,
    A: for<'b> ParseableParametrized<NlaBuffer<&'b [u8]>, &'b str>,
{
    fn parse(buf: &TcMessageBuffer<&'a T>) -> Result<Self, DecodeError> {
        Ok(Self {
            header: TcHeader::parse(buf).context("failed to parse tc message header")?,
            nlas: Vec::<Nla<A>>::parse(buf).context("failed to parse tc message NLAs")?,
        })
    }
}

impl<'a, T, A> Parseable<TcMessageBuffer<&'a T>> for Vec<Nla<A>>
where
    T: AsRef<[u8]>,
    A: for<'b> ParseableParametrized<NlaBuffer<&'b [u8]>, &'b str>,
{
    fn parse(buf: &TcMessageBuffer<&'a T>) -> Result<Self, DecodeError> {
        let mut nlas = vec![];
        let mut kind = String::new();

        for nla_buf in buf.nlas() {
            let buf = nla_buf?;
            let payload = buf.value();
            let nla = match buf.kind() {
                TCA_UNSPEC => Nla::Unspec(payload.to_vec()),
                TCA_KIND => {
                    kind = parse_string(payload)?;
                    Nla::Kind(kind.clone())
                }
                TCA_OPTIONS => Nla::Options(A::parse_with_param(&buf, &kind)?),
                TCA_STATS => Nla::Stats(Stats::parse(&StatsBuffer::new_checked(payload)?)?),
                TCA_XSTATS => Nla::XStats(payload.to_vec()),
                TCA_RATE => Nla::Rate(payload.to_vec()),
                TCA_FCNT => Nla::Fcnt(payload.to_vec()),
                TCA_STATS2 => {
                    let mut nlas = vec![];
                    for nla in NlasIterator::new(payload) {
                        nlas.push(Stats2::parse(&(nla?))?);
                    }
                    Nla::Stats2(nlas)
                }
                TCA_STAB => Nla::Stab(payload.to_vec()),
                TCA_CHAIN => Nla::Chain(payload.to_vec()),
                TCA_HW_OFFLOAD => Nla::HwOffload(parse_u8(payload)?),
                _ => Nla::Other(DefaultNla::parse(&buf)?),
            };

            nlas.push(nla);
        }
        Ok(nlas)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        constants::*,
        nlas::NlasIterator,
        tc::{Ingress, Nla, Qdisc, Stats, Stats2, StatsBuffer, TC_HEADER_LEN},
        traits::{Emitable, Parseable},
        TcHeader, TcMessage, TcMessageBuffer,
    };

    #[rustfmt::skip]
    static QDISC_INGRESS_PACKET: [u8; 136] = [
        0,       // family
        0, 0, 0, // pad1 + pad2
        84, 0, 0, 0, // Interface index = 84
        0, 0, 255, 255, // handle:  0xffff0000
        241, 255, 255, 255, // parent: 0xfffffff1
        1, 0, 0, 0, // info: refcnt: 1

        // nlas
        12, 0, // length
        1, 0,  // type: TCA_KIND
        105, 110, 103, 114, 101, 115, 115, 0, // ingress\0

        4, 0, // length
        2, 0, // type: TCA_OPTIONS

        5, 0, // length
        12, 0,// type: TCA_HW_OFFLOAD
        0,    // data: 0
        0, 0, 0,// padding

        48, 0, // length
        7, 0,  // type: TCA_STATS2
            20, 0, // length
            1, 0, // type: TCA_STATS_BASIC
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            24, 0,
            3, 0, // type: TCA_STATS_QUEUE
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,

        44, 0, // length
        3, 0,  // type: TCA_STATS
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ];

    #[test]
    fn tc_packet_header_read() {
        let packet = TcMessageBuffer::new(QDISC_INGRESS_PACKET);
        assert_eq!(packet.family(), 0);
        assert_eq!(packet.index(), 84);
        assert_eq!(packet.handle(), 0xffff0000);
        assert_eq!(packet.parent(), 0xfffffff1);
        assert_eq!(packet.info(), 1);
    }

    #[test]
    fn tc_packet_header_build() {
        let mut buf = vec![0xff; TC_HEADER_LEN];
        {
            let mut packet = TcMessageBuffer::new(&mut buf);
            packet.set_family(0);
            packet.set_pad1(0);
            packet.set_pad2(0);
            packet.set_index(84);
            packet.set_handle(0xffff0000);
            packet.set_parent(0xfffffff1);
            packet.set_info(1);
        }
        assert_eq!(&buf[..], &QDISC_INGRESS_PACKET[0..TC_HEADER_LEN]);
    }

    #[test]
    fn tc_packet_nlas_read() {
        let packet = TcMessageBuffer::new(&QDISC_INGRESS_PACKET[..]);
        assert_eq!(packet.nlas().count(), 5);
        let mut nlas = packet.nlas();

        let nla = nlas.next().unwrap().unwrap();
        nla.check_buffer_length().unwrap();
        assert_eq!(nla.length(), 12);
        assert_eq!(nla.kind(), TCA_KIND);
        assert_eq!(nla.value(), "ingress\0".as_bytes());

        let nla = nlas.next().unwrap().unwrap();
        nla.check_buffer_length().unwrap();
        assert_eq!(nla.length(), 4);
        assert_eq!(nla.kind(), TCA_OPTIONS);
        assert_eq!(nla.value(), []);

        let nla = nlas.next().unwrap().unwrap();
        nla.check_buffer_length().unwrap();
        assert_eq!(nla.length(), 5);
        assert_eq!(nla.kind(), TCA_HW_OFFLOAD);
        assert_eq!(nla.value(), [0]);

        let nla = nlas.next().unwrap().unwrap();
        nla.check_buffer_length().unwrap();
        assert_eq!(nla.length(), 48);
        assert_eq!(nla.kind(), TCA_STATS2);

        let mut stats2_iter = NlasIterator::new(nla.value());
        let stats2_nla = stats2_iter.next().unwrap().unwrap();
        stats2_nla.check_buffer_length().unwrap();
        assert_eq!(stats2_nla.length(), 20);
        assert_eq!(stats2_nla.kind(), TCA_STATS_BASIC);
        assert_eq!(stats2_nla.value(), [0; 16]);
        let s2 = Stats2::parse(&stats2_nla).unwrap();
        assert!(matches!(s2, Stats2::StatsBasic(_)));

        let stats2_nla = stats2_iter.next().unwrap().unwrap();
        stats2_nla.check_buffer_length().unwrap();
        assert_eq!(stats2_nla.length(), 24);
        assert_eq!(stats2_nla.kind(), TCA_STATS_QUEUE);
        assert_eq!(stats2_nla.value(), [0; 20]);
        let s2 = Stats2::parse(&stats2_nla).unwrap();
        assert!(matches!(s2, Stats2::StatsQueue(_)));

        let nla = nlas.next().unwrap().unwrap();
        nla.check_buffer_length().unwrap();
        assert_eq!(nla.length(), 44);
        assert_eq!(nla.kind(), TCA_STATS);
        assert_eq!(nla.value(), [0; 40]);
        let s = Stats::parse(&StatsBuffer::new(nla.value())).unwrap();
        assert_eq!(s.packets, 0);
        assert_eq!(s.backlog, 0);
    }

    #[test]
    fn tc_qdisc_ingress_emit() {
        let mut header = TcHeader::default();
        header.index = 84;
        header.handle = 0xffff0000;
        header.parent = 0xfffffff1;
        header.info = 1;

        let nlas = vec![
            Nla::Kind("ingress".into()),
            Nla::Options(Qdisc::Ingress(Ingress())),
        ];

        let msg = TcMessage::from_parts(header, nlas);
        let mut buf = vec![0; 36];
        assert_eq!(msg.buffer_len(), 36);
        msg.emit(&mut buf[..]);
        assert_eq!(&buf, &QDISC_INGRESS_PACKET[..36]);
    }

    #[test]
    fn tc_qdisc_ingress_read() {
        let packet = TcMessageBuffer::new_checked(&QDISC_INGRESS_PACKET).unwrap();

        let msg = TcMessage::<Qdisc>::parse(&packet).unwrap();
        assert_eq!(msg.header.index, 84);
        assert_eq!(msg.nlas.len(), 5);

        let nla = msg.nlas.iter().next().unwrap();
        assert_eq!(nla, &Nla::Kind(String::from("ingress")));
    }
}
