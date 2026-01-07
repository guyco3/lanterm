/// Macro to register games in the registry with automatic initializer generation
/// 
/// Usage in games/mod.rs:
/// ```
/// register_games! {
///     pong => {
///         types: (PongGame, PongAction, PongState),
///         id: "pong",
///         name: "Pong",
///         description: "Classic Pong game",
///         author: "LanTerm Team"
///     },
///     rand_num => {
///         types: (NumberGame, GuessAction, NumberState),
///         id: "rand_num",
///         name: "Number Guessing",
///         description: "Guess the number",
///         author: "LanTerm Team"
///     }
/// }
/// ```
#[macro_export]
macro_rules! register_games {
    (
        $(
            $module:ident => {
                types: ($game:ident, $action:ident, $state:ident),
                id: $id:expr,
                name: $name:expr,
                description: $desc:expr,
                author: $author:expr
            }
        ),* $(,)?
    ) => {
        /// Get all available games with their metadata and initializers
        pub fn get_all_games() -> Vec<GameRegistry> {
            vec![
                $(
                    GameRegistry {
                        info: GameInfo {
                            id: $id,
                            name: $name,
                            description: $desc,
                            author: $author,
                        },
                        initializer: |send, recv, conn, is_host, terminal, local_id| {
                            Box::pin(async move {
                                use $crate::core::{engine::Engine, network::NetworkManager};
                                use $crate::core::network::InternalMsg;
                                use $crate::games::$module::{$game, $action, $state};
                                
                                let network = NetworkManager::<InternalMsg<$action, $state>>::new(send, recv, conn, local_id);
                                let game = $game::new(is_host);
                                let engine = Engine::new(game, network, is_host);
                                engine.run(terminal).await
                            })
                        },
                    }
                ),*
            ]
        }
        
        /// Get a game by ID
        pub fn get_game(id: &str) -> Option<GameRegistry> {
            get_all_games().into_iter().find(|g| g.info.id == id)
        }
    };
}
