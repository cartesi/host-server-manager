// Copyright 2021 Cartesi Pte. Ltd.
//
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

pub mod grpc;
pub mod utils;

use grpc::proto::cartesi_machine::*;
use grpc::proto::host_server_manager::StartSessionRequest;

pub fn generate_default_machine_config(files_dir: &str) -> MachineConfig {
    MachineConfig {
        processor: Some(ProcessorConfig {
            x1_oneof: Some(processor_config::X1Oneof::X1(0)),
            x2_oneof: Some(processor_config::X2Oneof::X2(0)),
            x3_oneof: Some(processor_config::X3Oneof::X3(0)),
            x4_oneof: Some(processor_config::X4Oneof::X4(0)),
            x5_oneof: Some(processor_config::X5Oneof::X5(0)),
            x6_oneof: Some(processor_config::X6Oneof::X6(0)),
            x7_oneof: Some(processor_config::X7Oneof::X7(0)),
            x8_oneof: Some(processor_config::X8Oneof::X8(0)),
            x9_oneof: Some(processor_config::X9Oneof::X9(0)),
            x10_oneof: Some(processor_config::X10Oneof::X10(0)),
            x11_oneof: Some(processor_config::X11Oneof::X11(0)),
            x12_oneof: Some(processor_config::X12Oneof::X12(0)),
            x13_oneof: Some(processor_config::X13Oneof::X13(0)),
            x14_oneof: Some(processor_config::X14Oneof::X14(0)),
            x15_oneof: Some(processor_config::X15Oneof::X15(0)),
            x16_oneof: Some(processor_config::X16Oneof::X16(0)),
            x17_oneof: Some(processor_config::X17Oneof::X17(0)),
            x18_oneof: Some(processor_config::X18Oneof::X18(0)),
            x19_oneof: Some(processor_config::X19Oneof::X19(0)),
            x20_oneof: Some(processor_config::X20Oneof::X20(0)),
            x21_oneof: Some(processor_config::X21Oneof::X21(0)),
            x22_oneof: Some(processor_config::X22Oneof::X22(0)),
            x23_oneof: Some(processor_config::X23Oneof::X23(0)),
            x24_oneof: Some(processor_config::X24Oneof::X24(0)),
            x25_oneof: Some(processor_config::X25Oneof::X25(0)),
            x26_oneof: Some(processor_config::X26Oneof::X26(0)),
            x27_oneof: Some(processor_config::X27Oneof::X27(0)),
            x28_oneof: Some(processor_config::X28Oneof::X28(0)),
            x29_oneof: Some(processor_config::X29Oneof::X29(0)),
            x30_oneof: Some(processor_config::X30Oneof::X30(0)),
            x31_oneof: Some(processor_config::X31Oneof::X31(0)),
            pc_oneof: Some(processor_config::PcOneof::Pc(0x1000)),
            mvendorid_oneof: Some(processor_config::MvendoridOneof::Mvendorid(0x6361727465736920)),
            marchid_oneof: Some(processor_config::MarchidOneof::Marchid(0x7)),
            mimpid_oneof: Some(processor_config::MimpidOneof::Mimpid(1)),
            mcycle_oneof: Some(processor_config::McycleOneof::Mcycle(0)),
            minstret_oneof: Some(processor_config::MinstretOneof::Minstret(0)),
            mstatus_oneof: Some(processor_config::MstatusOneof::Mstatus(0)),
            mtvec_oneof: Some(processor_config::MtvecOneof::Mtvec(0)),
            mscratch_oneof: Some(processor_config::MscratchOneof::Mscratch(0)),
            mepc_oneof: Some(processor_config::MepcOneof::Mepc(0)),
            mcause_oneof: Some(processor_config::McauseOneof::Mcause(0)),
            mtval_oneof: Some(processor_config::MtvalOneof::Mtval(0)),
            misa_oneof: Some(processor_config::MisaOneof::Misa(0x141101)),
            mie_oneof: Some(processor_config::MieOneof::Mie(0)),
            mip_oneof: Some(processor_config::MipOneof::Mip(0)),
            medeleg_oneof: Some(processor_config::MedelegOneof::Medeleg(0)),
            mideleg_oneof: Some(processor_config::MidelegOneof::Mideleg(0)),
            mcounteren_oneof: Some(processor_config::McounterenOneof::Mcounteren(0)),
            stvec_oneof: Some(processor_config::StvecOneof::Stvec(0)),
            sscratch_oneof: Some(processor_config::SscratchOneof::Sscratch(0)),
            sepc_oneof: Some(processor_config::SepcOneof::Sepc(0)),
            scause_oneof: Some(processor_config::ScauseOneof::Scause(0)),
            stval_oneof: Some(processor_config::StvalOneof::Stval(0)),
            satp_oneof: Some(processor_config::SatpOneof::Satp(0)),
            scounteren_oneof: Some(processor_config::ScounterenOneof::Scounteren(0)),
            ilrsc_oneof: Some(processor_config::IlrscOneof::Ilrsc(u64::MAX)),
            iflags_oneof: Some(processor_config::IflagsOneof::Iflags(0x0)),
        }),
        ram: Some(RamConfig {
            length: 64 << 20,
            image_filename: format!("{}/linux.bin", files_dir),
        }),
        rom: Some(RomConfig {
            bootargs: String::from("console=hvc0 rootfstype=ext2 root=/dev/mtdblock0 rwmtdparts=flash.0:-(rootfs) -- for i in $(seq 0 5 1000); do yield progress $i; done"),
            image_filename: format!("{}/rom.bin", files_dir),
        }),
        flash_drive: vec![MemoryRangeConfig {
            start: 1 << 63,
            length: 62914560,
            image_filename: format!("{}/rootfs.ext2", files_dir),
            shared: false,
        }],
        clint: Some(ClintConfig {
            mtimecmp_oneof: Some(clint_config::MtimecmpOneof::Mtimecmp(0)),
        }),
        htif: Some(HtifConfig {
            console_getchar: false,
            yield_manual: true,
            yield_automatic: false,
            fromhost_oneof: Some(htif_config::FromhostOneof::Fromhost(0)),
            tohost_oneof: Some(htif_config::TohostOneof::Tohost(0)),
        }),
        dhd: Some(DhdConfig {
            tstart: 0,
            tlength: 0,
            image_filename: String::new(),
            dlength: 0,
            hlength: 0,
            h: vec![0; 4],
        }),
        rollup: Some(RollupConfig {
            rx_buffer: Some(MemoryRangeConfig {
                start: 0x60000000,
                length: 2 << 20,
                image_filename: "".to_string(),
                shared: false
            }),
            tx_buffer: Some(MemoryRangeConfig {
                start: 0x60200000,
                length: 2 << 20,
                image_filename: "".to_string(),
                shared: false
            }),
            input_metadata: Some(MemoryRangeConfig {
                start: 0x60400000,
                length: 4096,
                image_filename: "".to_string(),
                shared: false
            }),
            voucher_hashes: Some(MemoryRangeConfig {
                start: 0x60600000,
                length: 2 << 20,
                image_filename: "".to_string(),
                shared: false
            }),
            notice_hashes: Some(MemoryRangeConfig {
                start: 0x60800000,
                length: 2 << 20,
                image_filename: "".to_string(),
                shared: false
            }),
        }),
    }
}

pub fn generate_default_machine_rt_config() -> MachineRuntimeConfig {
    MachineRuntimeConfig {
        dhd: None,
        concurrency: None,
    }
}

pub fn generate_default_start_session_request(
    files_dir: &str,
    session_id: &str,
) -> StartSessionRequest {
    StartSessionRequest {
        session_id: session_id.to_string(),
        machine: Some(MachineRequest {
            runtime: Some(generate_default_machine_rt_config()),
            machine_oneof: Some(machine_request::MachineOneof::Config(
                generate_default_machine_config(files_dir),
            )),
        }),
        active_epoch_index: 0,
        server_cycles: None,
        server_deadline: None,
    }
}
