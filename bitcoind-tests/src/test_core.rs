use bitcoincore_rpc::RpcApi;

use crate::TestNode;

#[test]
fn smoke_starts_and_mines() {
    let tn = TestNode::start().expect("start/attach node");
    let info = tn.rpc.get_blockchain_info().expect("rpc works");
    assert!(info.blocks >= 101);
}
