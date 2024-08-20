use alloy_consensus::{SidecarBuilder, SimpleCoder};
use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_node_bindings::Anvil;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use ethers::{
    middleware::Middleware,
    providers::{Ipc, Provider as LegacyProvider},
};
use futures::stream::StreamExt;

#[tokio::main]
async fn main() {
    let anvil = Anvil::new()
        .args(["--ipc", "/tmp/anvil.ipc"])
        .try_spawn()
        .unwrap();

    let ipc = Ipc::connect("/tmp/anvil.ipc").await.unwrap();
    let provider = LegacyProvider::new(ipc);
    let mut stream = provider.subscribe_pending_txs().await.unwrap();

    tokio::spawn(async move {
        let signer: PrivateKeySigner = anvil.keys()[0].clone().into();
        let wallet = EthereumWallet::from(signer);

        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(anvil.endpoint().parse().unwrap());

        let alice = anvil.addresses()[0];

        let sidecar: SidecarBuilder<SimpleCoder> =
            SidecarBuilder::from_slice("Blobs are fun!".as_bytes());
        let sidecar = sidecar.build().unwrap();

        let gas_price = provider.get_gas_price().await.unwrap();
        let eip1559_est = provider.estimate_eip1559_fees(None).await.unwrap();

        let tx = TransactionRequest::default()
            .with_to(alice)
            .with_max_fee_per_blob_gas(gas_price)
            .with_max_fee_per_gas(eip1559_est.max_fee_per_gas)
            .with_max_priority_fee_per_gas(eip1559_est.max_priority_fee_per_gas)
            .with_blob_sidecar(sidecar);

        loop {
            let _ = provider.send_transaction(tx.clone()).await.unwrap();
        }
    });

    loop {
        let tx_hash = stream.next().await.unwrap();

        let raw_tx: Option<String> = provider
            .request("eth_getRawTransactionByHash", vec![tx_hash])
            .await
            .unwrap();

        if let Some(raw_tx) = raw_tx {
            if raw_tx.len() > 300 {
                println!("Blob found");
            } else {
                println!("Blob not found");
            }
        }
    }
}
