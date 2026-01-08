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
                        initializer: |net_lobby, is_host, terminal| {
                            Box::pin(async move {
                                use $crate::core::engine::Engine;
                                use $crate::core::network::InternalMsg;
                                use $crate::games::$module::{$game, $action, $state};

                                // Fetch local id from lobby manager and pass into the game constructor
                                let my_id = net_lobby.local_id();

                                // Upgrade to typed active manager
                                let typed_net = net_lobby.upgrade::<InternalMsg<$action, $state>>();

                                // Initialize game with host flag and local endpoint id
                                let game = $game::new(is_host, my_id);
                                let engine = Engine::new(game, typed_net, is_host);
                                let finished_net = engine.run(terminal).await?;
                                Ok(finished_net.reset())
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
