use mio_glue::player::Player;

use crate::*;

impl MioFrontendWeak {
    pub fn start_player_thread(&self) {
        let p = self.player.clone();
        let a = self.app.clone();
        tokio::spawn(async move {
            let ls = LocalSet::new();
            ls.spawn_local(player_thread(p, a)).await;
        });
    }
}

async fn player_thread(w_player: StdWeak<Player>, w_app: SlWeak<TopLevelWindow>) {
    let player = w_player
        .upgrade()
        .ok_or(error::Error::StrongGonePlayer)
        .unwrap();

}
