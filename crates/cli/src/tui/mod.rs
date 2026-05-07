mod dashboard;
mod splash;

use crate::client::RuneClient;

pub async fn run(client: RuneClient) -> anyhow::Result<()> {
    match splash::check(&client).await? {
        splash::SplashResult::Ready => dashboard::run(client).await,
        splash::SplashResult::Offline(_) => Ok(()), // error already shown on screen
    }
}
