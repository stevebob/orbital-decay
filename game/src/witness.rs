use crate::{player, ActionError, Config, ExternalEvent, GameControlFlow, Input};
use direction::CardinalDirection;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct Game {
    inner_game: crate::Game,
}

#[derive(Serialize, Deserialize)]
pub struct RunningGame {
    game: crate::Game,
}

impl RunningGame {
    pub fn new(game: Game, running: Running) -> Self {
        let _ = running;
        Self {
            game: game.inner_game,
        }
    }

    pub fn into_game(self) -> (Game, Running) {
        (
            Game {
                inner_game: self.game,
            },
            Running(Private),
        )
    }
}

struct Private;

pub struct Running(Private);
pub struct Upgrade(Private);
pub struct GetRangedWeapon(Private);
pub struct GetMeleeWeapon(Private);

pub enum Witness {
    Running(Running),
    Upgrade(Upgrade),
    GetRangedWeapon(GetRangedWeapon),
    GetMeleeWeapon(GetMeleeWeapon),
    GameOver,
}

impl Witness {
    fn running(private: Private) -> Self {
        Self::Running(Running(private))
    }
    fn upgrade(private: Private) -> Self {
        Self::Upgrade(Upgrade(private))
    }
}

pub enum ControlInput {
    Walk(CardinalDirection),
    Wait,
}

pub fn new_game<R: Rng>(config: &Config, base_rng: &mut R) -> (Game, Running) {
    let g = Game {
        inner_game: crate::Game::new(config, base_rng),
    };
    (g, Running(Private))
}

impl Running {
    pub fn new_panics() -> Self {
        panic!("this constructor is meant for temporary use during debugging to get the code to compile")
    }

    pub fn into_witness(self) -> Witness {
        Witness::Running(self)
    }

    pub fn tick(self, game: &mut Game, since_last_tick: Duration, config: &Config) -> Witness {
        use GameControlFlow::*;
        let Self(private) = self;
        match game.inner_game.handle_tick(since_last_tick, config) {
            None => Witness::running(private),
            Some(Upgrade) => Witness::upgrade(private),
            Some(GameOver) => Witness::GameOver,
            Some(_) => todo!(),
        }
    }

    pub fn walk(
        self,
        game: &mut Game,
        direction: CardinalDirection,
        config: &Config,
    ) -> (Witness, Result<(), ActionError>) {
        let Self(private) = self;
        game.witness_handle_input(Input::Walk(direction), config, private)
    }

    pub fn wait(self, game: &mut Game, config: &Config) -> (Witness, Result<(), ActionError>) {
        let Self(private) = self;
        game.witness_handle_input(Input::Wait, config, private)
    }

    pub fn get(self, game: &Game) -> (Witness, Result<(), ActionError>) {
        if let Some(weapon) = game.inner_ref().weapon_under_player() {
            if weapon.is_ranged() {
                let Self(private) = self;
                return (Witness::GetRangedWeapon(GetRangedWeapon(private)), Ok(()));
            }
            if weapon.is_melee() {
                let Self(private) = self;
                return (Witness::GetMeleeWeapon(GetMeleeWeapon(private)), Ok(()));
            }
        }
        (self.into_witness(), Err(ActionError::NoItemToGet))
    }
}

impl Upgrade {
    pub fn upgrade(
        self,
        game: &mut Game,
        upgrade: player::Upgrade,
        config: &Config,
    ) -> (Witness, Result<(), ActionError>) {
        let Self(private) = self;
        let input = Input::Upgrade(upgrade);
        game.witness_handle_input(input, config, private)
    }

    pub fn cancel(self) -> Witness {
        let Self(private) = self;
        Witness::running(private)
    }
}

impl GetRangedWeapon {
    pub fn commit(self, game: &mut Game, slot: player::RangedWeaponSlot) -> Witness {
        game.inner_game.player_equip_ranged_weapon_from_ground(slot);
        let Self(private) = self;
        Witness::running(private)
    }

    pub fn cancel(self) -> Witness {
        let Self(private) = self;
        Witness::running(private)
    }
}

impl GetMeleeWeapon {
    pub fn commit(self, game: &mut Game) -> Witness {
        game.inner_game.player_equip_melee_weapon_from_ground();
        let Self(private) = self;
        Witness::running(private)
    }

    pub fn cancel(self) -> Witness {
        let Self(private) = self;
        Witness::running(private)
    }
}

impl Game {
    fn witness_handle_input(
        &mut self,
        input: Input,
        config: &Config,
        private: Private,
    ) -> (Witness, Result<(), ActionError>) {
        use GameControlFlow::*;
        match self.inner_game.handle_input(input, config) {
            Err(e) => (Witness::running(private), Err(e)),
            Ok(None) => (Witness::running(private), Ok(())),
            Ok(Some(Upgrade)) => (Witness::upgrade(private), Ok(())),
            Ok(Some(GameOver)) => (Witness::GameOver, Ok(())),
            Ok(Some(_)) => todo!(),
        }
    }

    pub fn inner_ref(&self) -> &crate::Game {
        &self.inner_game
    }

    pub fn into_running_game(self, running: Running) -> RunningGame {
        RunningGame::new(self, running)
    }

    pub fn npc_turn(&mut self) {
        self.inner_game.handle_npc_turn()
    }

    pub fn events(&mut self) -> impl '_ + Iterator<Item = ExternalEvent> {
        self.inner_game.events()
    }

    pub fn player(&self) -> &player::Player {
        self.inner_game.player()
    }
}
