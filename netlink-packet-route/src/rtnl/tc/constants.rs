// SPDX-License-Identifier: MIT

/// Handles
pub const TC_H_MAJ_MASK: u32 = 0xFFFF0000;
pub const TC_H_MIN_MASK: u32 = 0x0000FFFF;

pub const TC_H_UNSPEC: u32 = 0;
pub const TC_H_ROOT: u32 = 0xFFFFFFFF;
pub const TC_H_INGRESS: u32 = 0xFFFFFFF1;
pub const TC_H_CLSACT: u32 = TC_H_INGRESS;
