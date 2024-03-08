use crate::MioFrontendWeak;

impl MioFrontendWeak {
    pub fn start_album_poll_task(&self) -> tokio::task::JoinHandle<()> {
        self.w_rt().unwrap().spawn(self.clone().album_poll_task())
    }

    async fn album_poll_task(self) {}
}
