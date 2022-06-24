// Copyright 2022 Cartesi Pte. Ltd.
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

use crate::common::*;

#[tokio::test]
#[serial_test::serial]
async fn test_it_get_epoch_status_of_empty_epoch() {
    let _manager = manager::Wrapper::new().await;
    let mut grpc_client = grpc_client::connect().await;
    grpc_client
        .start_session(grpc_client::create_start_session_request("rollup session"))
        .await
        .unwrap();
    let response = grpc_client
        .get_epoch_status(grpc_client::GetEpochStatusRequest {
            session_id: "rollup session".into(),
            epoch_index: 0,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        response,
        grpc_client::GetEpochStatusResponse {
            session_id: "rollup session".into(),
            epoch_index: 0,
            state: grpc_client::EpochState::Active as i32,
            most_recent_machine_hash: None,
            most_recent_vouchers_epoch_root_hash: Some(decode_hash(
                "cf277fb80a82478460e8988570b718f1e083ceb76f7e271a1a1497e5975f53ae"
            )),
            most_recent_notices_epoch_root_hash: Some(decode_hash(
                "cf277fb80a82478460e8988570b718f1e083ceb76f7e271a1a1497e5975f53ae"
            )),
            processed_inputs: vec![],
            pending_input_count: 0,
            taint_status: None,
        }
    );
}

#[tokio::test]
#[serial_test::serial]
async fn test_it_get_epoch_status_of_epoch_with_voucher_notice_and_report() {
    let _manager = manager::Wrapper::new().await;
    let mut grpc_client = grpc_client::connect().await;
    setup_advance_state(&mut grpc_client, "rollup session").await;
    let address = String::from("0x") + &"fa".repeat(20);
    http_client::insert_voucher(address, "0xdeadbeef".into())
        .await
        .unwrap();
    http_client::insert_notice("0xdeadbeef".into())
        .await
        .unwrap();
    http_client::insert_report("0xdeadbeef".into())
        .await
        .unwrap();
    finish_advance_state(&mut grpc_client, "rollup session").await;
    let response = grpc_client
        .get_epoch_status(grpc_client::GetEpochStatusRequest {
            session_id: "rollup session".into(),
            epoch_index: 0,
        })
        .await
        .unwrap()
        .into_inner();
    let expected = grpc_client::GetEpochStatusResponse {
        session_id: "rollup session".into(),
        epoch_index: 0,
        state: grpc_client::EpochState::Active as i32,
        most_recent_machine_hash: None,
        most_recent_vouchers_epoch_root_hash: Some(decode_hash(
            "29676ea41aaf54b4d66d45bc60b9c8f71b5f9166035d375626746e7396baa7a1",
        )),
        most_recent_notices_epoch_root_hash: Some(decode_hash(
            "63a367741b1feb9c2dc64bda8ac4a083ebbe5fd1f7bb4746e94597c988f30197",
        )),
        processed_inputs: vec![grpc_client::ProcessedInput {
            input_index: 0,
            most_recent_machine_hash: None,
            voucher_hashes_in_epoch: Some(grpc_client::MerkleTreeProof {
                target_address: 0,
                log2_target_size: 5,
                target_hash: Some(decode_hash(
                    "bf21d3dd50b9c5e542ea86c0f555b1bde6373829b59f51afd4a95eef24f05245",
                )),
                log2_root_size: 37,
                root_hash: Some(decode_hash(
                    "29676ea41aaf54b4d66d45bc60b9c8f71b5f9166035d375626746e7396baa7a1",
                )),
                sibling_hashes: vec![
                    decode_hash("78ccaaab73373552f207a63599de54d7d8d0c1805f86ce7da15818d09f4cff62"),
                    decode_hash("8f6162fa308d2b3a15dc33cffac85f13ab349173121645aedf00f471663108be"),
                    decode_hash("7e275adf313a996c7e2950cac67caba02a5ff925ebf9906b58949f3e77aec5b9"),
                    decode_hash("7fa06ba11241ddd5efdc65d4e39c9f6991b74fd4b81b62230808216c876f827c"),
                    decode_hash("0ff273fcbf4ae0f2bd88d6cf319ff4004f8d7dca70d4ced4e74d2c74139739e6"),
                    decode_hash("c5ab8111456b1f28f3c7a0a604b4553ce905cb019c463ee159137af83c350b22"),
                    decode_hash("fffc43bd08273ccf135fd3cacbeef055418e09eb728d727c4d5d5c556cdea7e3"),
                    decode_hash("1c25ef10ffeb3c7d08aa707d17286e0b0d3cbcb50f1bd3b6523b63ba3b52dd0f"),
                    decode_hash("6ca6a3f763a9395f7da16014725ca7ee17e4815c0ff8119bf33f273dee11833b"),
                    decode_hash("6075c657a105351e7f0fce53bc320113324a522e8fd52dc878c762551e01a46e"),
                    decode_hash("edf260291f734ddac396a956127dde4c34c0cfb8d8052f88ac139658ccf2d507"),
                    decode_hash("44a6d974c75b07423e1d6d33f481916fdd45830aea11b6347e700cd8b9f0767c"),
                    decode_hash("4f05f4acb83f5b65168d9fef89d56d4d77b8944015e6b1eed81b0238e2d0dba3"),
                    decode_hash("504364a5c6858bf98fff714ab5be9de19ed31a976860efbd0e772a2efe23e2e0"),
                    decode_hash("e2e7610b87a5fdf3a72ebe271287d923ab990eefac64b6e59d79f8b7e08c46e3"),
                    decode_hash("776a31db34a1a0a7caaf862cffdfff1789297ffadc380bd3d39281d340abd3ad"),
                    decode_hash("2def10d13dd169f550f578bda343d9717a138562e0093b380a1120789d53cf10"),
                    decode_hash("4ebfd9cd7bca2505f7bef59cc1c12ecc708fff26ae4af19abe852afe9e20c862"),
                    decode_hash("a2fca4a49658f9fab7aa63289c91b7c7b6c832a6d0e69334ff5b0a3483d09dab"),
                    decode_hash("ad676aa337a485e4728a0b240d92b3ef7b3c372d06d189322bfd5f61f1e7203e"),
                    decode_hash("3d04cffd8b46a874edf5cfae63077de85f849a660426697b06a829c70dd1409c"),
                    decode_hash("e026cc5a4aed3c22a58cbd3d2ac754c9352c5436f638042dca99034e83636516"),
                    decode_hash("7ad66c0a68c72cb89e4fb4303841966e4062a76ab97451e3b9fb526a5ceb7f82"),
                    decode_hash("e1cea92ed99acdcb045a6726b2f87107e8a61620a232cf4d7d5b5766b3952e10"),
                    decode_hash("292c23a9aa1d8bea7e2435e555a4a60e379a5a35f3f452bae60121073fb6eead"),
                    decode_hash("617bdd11f7c0a11f49db22f629387a12da7596f9d1704d7465177c63d88ec7d7"),
                    decode_hash("defff6d330bb5403f63b14f33b578274160de3a50df4efecf0e0db73bcdd3da5"),
                    decode_hash("ecd50eee38e386bd62be9bedb990706951b65fe053bd9d8a521af753d139e2da"),
                    decode_hash("3b8ec09e026fdc305365dfc94e189a81b38c7597b3d941c279f042e8206e0bd8"),
                    decode_hash("890740a8eb06ce9be422cb8da5cdafc2b58c0a5e24036c578de2a433c828ff7d"),
                    decode_hash("633dc4d7da7256660a892f8f1604a44b5432649cc8ec5cb3ced4c4e6ac94dd1d"),
                    decode_hash("290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e563"),
                ],
            }),
            notice_hashes_in_epoch: Some(grpc_client::MerkleTreeProof {
                target_address: 0,
                log2_target_size: 5,
                target_hash: Some(decode_hash(
                    "660c2d35b0a43d8179792345211d0eab28d88f47fafadd8334b80196cad41ded",
                )),
                log2_root_size: 37,
                root_hash: Some(decode_hash(
                    "63a367741b1feb9c2dc64bda8ac4a083ebbe5fd1f7bb4746e94597c988f30197",
                )),
                sibling_hashes: vec![
                    decode_hash("78ccaaab73373552f207a63599de54d7d8d0c1805f86ce7da15818d09f4cff62"),
                    decode_hash("8f6162fa308d2b3a15dc33cffac85f13ab349173121645aedf00f471663108be"),
                    decode_hash("7e275adf313a996c7e2950cac67caba02a5ff925ebf9906b58949f3e77aec5b9"),
                    decode_hash("7fa06ba11241ddd5efdc65d4e39c9f6991b74fd4b81b62230808216c876f827c"),
                    decode_hash("0ff273fcbf4ae0f2bd88d6cf319ff4004f8d7dca70d4ced4e74d2c74139739e6"),
                    decode_hash("c5ab8111456b1f28f3c7a0a604b4553ce905cb019c463ee159137af83c350b22"),
                    decode_hash("fffc43bd08273ccf135fd3cacbeef055418e09eb728d727c4d5d5c556cdea7e3"),
                    decode_hash("1c25ef10ffeb3c7d08aa707d17286e0b0d3cbcb50f1bd3b6523b63ba3b52dd0f"),
                    decode_hash("6ca6a3f763a9395f7da16014725ca7ee17e4815c0ff8119bf33f273dee11833b"),
                    decode_hash("6075c657a105351e7f0fce53bc320113324a522e8fd52dc878c762551e01a46e"),
                    decode_hash("edf260291f734ddac396a956127dde4c34c0cfb8d8052f88ac139658ccf2d507"),
                    decode_hash("44a6d974c75b07423e1d6d33f481916fdd45830aea11b6347e700cd8b9f0767c"),
                    decode_hash("4f05f4acb83f5b65168d9fef89d56d4d77b8944015e6b1eed81b0238e2d0dba3"),
                    decode_hash("504364a5c6858bf98fff714ab5be9de19ed31a976860efbd0e772a2efe23e2e0"),
                    decode_hash("e2e7610b87a5fdf3a72ebe271287d923ab990eefac64b6e59d79f8b7e08c46e3"),
                    decode_hash("776a31db34a1a0a7caaf862cffdfff1789297ffadc380bd3d39281d340abd3ad"),
                    decode_hash("2def10d13dd169f550f578bda343d9717a138562e0093b380a1120789d53cf10"),
                    decode_hash("4ebfd9cd7bca2505f7bef59cc1c12ecc708fff26ae4af19abe852afe9e20c862"),
                    decode_hash("a2fca4a49658f9fab7aa63289c91b7c7b6c832a6d0e69334ff5b0a3483d09dab"),
                    decode_hash("ad676aa337a485e4728a0b240d92b3ef7b3c372d06d189322bfd5f61f1e7203e"),
                    decode_hash("3d04cffd8b46a874edf5cfae63077de85f849a660426697b06a829c70dd1409c"),
                    decode_hash("e026cc5a4aed3c22a58cbd3d2ac754c9352c5436f638042dca99034e83636516"),
                    decode_hash("7ad66c0a68c72cb89e4fb4303841966e4062a76ab97451e3b9fb526a5ceb7f82"),
                    decode_hash("e1cea92ed99acdcb045a6726b2f87107e8a61620a232cf4d7d5b5766b3952e10"),
                    decode_hash("292c23a9aa1d8bea7e2435e555a4a60e379a5a35f3f452bae60121073fb6eead"),
                    decode_hash("617bdd11f7c0a11f49db22f629387a12da7596f9d1704d7465177c63d88ec7d7"),
                    decode_hash("defff6d330bb5403f63b14f33b578274160de3a50df4efecf0e0db73bcdd3da5"),
                    decode_hash("ecd50eee38e386bd62be9bedb990706951b65fe053bd9d8a521af753d139e2da"),
                    decode_hash("3b8ec09e026fdc305365dfc94e189a81b38c7597b3d941c279f042e8206e0bd8"),
                    decode_hash("890740a8eb06ce9be422cb8da5cdafc2b58c0a5e24036c578de2a433c828ff7d"),
                    decode_hash("633dc4d7da7256660a892f8f1604a44b5432649cc8ec5cb3ced4c4e6ac94dd1d"),
                    decode_hash("290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e563"),
                ],
            }),
            reports: vec![grpc_client::Report {
                payload: vec![222, 173, 190, 239],
            }],
            status: grpc_client::CompletionStatus::Accepted as i32,
            processed_input_one_of: Some(
                grpc_client::processed_input::ProcessedInputOneOf::AcceptedData(
                    grpc_client::AcceptedData {
                        voucher_hashes_in_machine: None,
                        vouchers: vec![grpc_client::Voucher {
                        keccak: Some(decode_hash(
                            "93d99a3fafa9fbbeb13bf691823046b8587e55f59d74ad9f22576becc9d3cdd2",
                        )),
                        address: Some(grpc_client::Address {
                            data: vec![
                                250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250,
                                250, 250, 250, 250, 250, 250, 250,
                            ],
                        }),
                        payload: vec![222, 173, 190, 239],
                        keccak_in_voucher_hashes: Some(grpc_client::MerkleTreeProof {
                            target_address: 0,
                            log2_target_size: 5,
                            target_hash: Some(decode_hash(
                                "b9f1453eb9ed39ec6097bd8aa7f1ef7fa898419bf716d5b9e14761d16ab2ae84",
                            )),
                            log2_root_size: 21,
                            root_hash: Some(decode_hash(
                                "bf21d3dd50b9c5e542ea86c0f555b1bde6373829b59f51afd4a95eef24f05245",
                            )),
                            sibling_hashes: vec![
                            decode_hash(
                                "99af665835aabfdc6740c7e2c3791a31c3cdc9f5ab962f681b12fc092816a62f",
                            ),
                            decode_hash(
                                "2b573c267a712a52e1d06421fe276a03efb1889f337201110fdc32a81f8e1524",
                            ),
                            decode_hash(
                                "7a71f6ee264c5d761379b3d7d617ca83677374b49d10aec50505ac087408ca89",
                            ),
                            decode_hash(
                                "f7549f26cc70ed5e18baeb6c81bb0625cb95bb4019aeecd40774ee87ae29ec51",
                            ),
                            decode_hash(
                                "2122e31e4bbd2b7c783d79cc30f60c6238651da7f0726f767d22747264fdb046",
                            ),
                            decode_hash(
                                "91e3eee5ca7a3da2b3053c9770db73599fb149f620e3facef95e947c0ee860b7",
                            ),
                            decode_hash(
                                "63e8806fa0d4b197a259e8c3ac28864268159d0ac85f8581ca28fa7d2c0c03eb",
                            ),
                            decode_hash(
                                "c9695393027fb106a8153109ac516288a88b28a93817899460d6310b71cf1e61",
                            ),
                            decode_hash(
                                "d8b96e5b7f6f459e9cb6a2f41bf276c7b85c10cd4662c04cbbb365434726c0a0",
                            ),
                            decode_hash(
                                "cd5deac729d0fdaccc441d09d7325f41586ba13c801b7eccae0f95d8f3933efe",
                            ),
                            decode_hash(
                                "30b0b9deb73e155c59740bacf14a6ff04b64bb8e201a506409c3fe381ca4ea90",
                            ),
                            decode_hash(
                                "8e7a427fa943d9966b389f4f257173676090c6e95f43e2cb6d65f8758111e309",
                            ),
                            decode_hash(
                                "c37b8b13ca95166fb7af16988a70fcc90f38bf9126fd833da710a47fb37a55e6",
                            ),
                            decode_hash(
                                "17d2dd614cddaa4d879276b11e0672c9560033d3e8453a1d045339d34ba601b9",
                            ),
                            decode_hash(
                                "3fc9a15f5b4869c872f81087bb6104b7d63e6f9ab47f2c43f3535eae7172aa7f",
                            ),
                            decode_hash(
                                "ae39ce8537aca75e2eff3e38c98011dfe934e700a0967732fc07b430dd656a23",
                            ),
                        ],
                        }),
                    }],
                        notice_hashes_in_machine: None,
                        notices: vec![grpc_client::Notice {
                        keccak: Some(decode_hash(
                            "b15ecb08bb827d74358e99df23d69a2962e5a16c3c3dfd80f365078bf9a29f1d",
                        )),
                        payload: vec![222, 173, 190, 239],
                        keccak_in_notice_hashes: Some(grpc_client::MerkleTreeProof {
                            target_address: 0,
                            log2_target_size: 5,
                            target_hash: Some(decode_hash(
                                "0e570852524a76e50c0afc6bae39698146c8ad68a62cb1472216026db2698400",
                            )),
                            log2_root_size: 21,
                            root_hash: Some(decode_hash(
                                "660c2d35b0a43d8179792345211d0eab28d88f47fafadd8334b80196cad41ded",
                            )),
                            sibling_hashes: vec![
                            decode_hash(
                                "99af665835aabfdc6740c7e2c3791a31c3cdc9f5ab962f681b12fc092816a62f",
                            ),
                            decode_hash(
                                "2b573c267a712a52e1d06421fe276a03efb1889f337201110fdc32a81f8e1524",
                            ),
                            decode_hash(
                                "7a71f6ee264c5d761379b3d7d617ca83677374b49d10aec50505ac087408ca89",
                            ),
                            decode_hash(
                                "f7549f26cc70ed5e18baeb6c81bb0625cb95bb4019aeecd40774ee87ae29ec51",
                            ),
                            decode_hash(
                                "2122e31e4bbd2b7c783d79cc30f60c6238651da7f0726f767d22747264fdb046",
                            ),
                            decode_hash(
                                "91e3eee5ca7a3da2b3053c9770db73599fb149f620e3facef95e947c0ee860b7",
                            ),
                            decode_hash(
                                "63e8806fa0d4b197a259e8c3ac28864268159d0ac85f8581ca28fa7d2c0c03eb",
                            ),
                            decode_hash(
                                "c9695393027fb106a8153109ac516288a88b28a93817899460d6310b71cf1e61",
                            ),
                            decode_hash(
                                "d8b96e5b7f6f459e9cb6a2f41bf276c7b85c10cd4662c04cbbb365434726c0a0",
                            ),
                            decode_hash(
                                "cd5deac729d0fdaccc441d09d7325f41586ba13c801b7eccae0f95d8f3933efe",
                            ),
                            decode_hash(
                                "30b0b9deb73e155c59740bacf14a6ff04b64bb8e201a506409c3fe381ca4ea90",
                            ),
                            decode_hash(
                                "8e7a427fa943d9966b389f4f257173676090c6e95f43e2cb6d65f8758111e309",
                            ),
                            decode_hash(
                                "c37b8b13ca95166fb7af16988a70fcc90f38bf9126fd833da710a47fb37a55e6",
                            ),
                            decode_hash(
                                "17d2dd614cddaa4d879276b11e0672c9560033d3e8453a1d045339d34ba601b9",
                            ),
                            decode_hash(
                                "3fc9a15f5b4869c872f81087bb6104b7d63e6f9ab47f2c43f3535eae7172aa7f",
                            ),
                            decode_hash(
                                "ae39ce8537aca75e2eff3e38c98011dfe934e700a0967732fc07b430dd656a23",
                            ),
                        ],
                        }),
                    }],
                    },
                ),
            ),
        }],
        pending_input_count: 0,
        taint_status: None,
    };
    assert_eq!(response, expected);
}

#[tokio::test]
#[serial_test::serial]
async fn test_it_fails_to_get_non_existent_epoch_status() {
    let _manager = manager::Wrapper::new().await;
    let mut grpc_client = grpc_client::connect().await;
    grpc_client
        .start_session(grpc_client::create_start_session_request("rollup session"))
        .await
        .unwrap();
    let err = grpc_client
        .get_epoch_status(grpc_client::GetEpochStatusRequest {
            session_id: "rollup session".into(),
            epoch_index: 123,
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

fn decode_hash(s: &str) -> grpc_client::Hash {
    grpc_client::Hash {
        data: hex::decode(s).unwrap(),
    }
}
