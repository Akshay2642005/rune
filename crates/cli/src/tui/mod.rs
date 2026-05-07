pub mod dashboard;

use crate::client::RuneClient;

pub async fn run(client: RuneClient) -> anyhow::Result<()> {
    dashboard::run(client).await
}
