use godot::classes::{
    AnimationPlayer, Button, CharacterBody2D, Control, ICharacterBody2D, Node2D, Sprite2D,
};
use godot::prelude::*;

struct GdExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GdExtension {}

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
struct Player {
    #[export]
    speed: f64,
    base: Base<CharacterBody2D>,
    sprite: Option<Gd<Sprite2D>>,
    animation_player: Option<Gd<AnimationPlayer>>,
}

#[godot_api]
impl ICharacterBody2D for Player {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Player {
            speed: 250.0,
            base,
            sprite: None,
            animation_player: None,
        }
    }
    fn ready(&mut self) {
        let sprite = self.base().get_node_as::<Sprite2D>("Sprite");
        self.sprite = Some(sprite);

        let animation_player = self
            .base()
            .get_node_as::<AnimationPlayer>("AnimationPlayer");
        self.animation_player = Some(animation_player);
    }

    fn physics_process(&mut self, _delta: f64) {
        let mut down = true;
        let mut left = true;
        let mut direction = Vector2::new(0.0, 0.0);

        let input = Input::singleton();
        if input.is_action_pressed("right") {
            direction.x += 1.0;
            left = false;
        }
        if input.is_action_pressed("left") {
            direction.x -= 1.0;
        }
        if input.is_action_pressed("up") {
            direction.y -= 1.0;
            down = false;
        }
        if input.is_action_pressed("down") {
            direction.y += 1.0;
        }

        if !direction.is_zero_approx() {
            let animation_name = match (down, left) {
                (true, true) => "downleft_idle",
                (true, false) => "downright_idle",
                (false, true) => "upleft_idle",
                (false, false) => "upright_idle",
            };
            if self
                .animation_player
                .as_ref()
                .unwrap()
                .get_current_animation()
                != animation_name.into()
            {
                self.animation_player
                    .as_mut()
                    .unwrap()
                    .set_current_animation(animation_name);
            }
        }

        let velocity = direction.normalized_or_zero() * self.speed as f32;
        self.base_mut().set_velocity(velocity);

        self.base_mut().move_and_slide();
    }
}

#[godot_api]
impl Player {
    #[func]
    fn increase_speed(&mut self, amount: f64) {
        self.speed += amount;
        self.base_mut().emit_signal("speed_increased", &[]);
    }

    #[signal]
    fn speed_increased(&self);
}

#[derive(Default, Debug)]
enum BeastOwner {
    #[default]
    Player,
    Enemy,
}

#[derive(Default, Debug)]
enum MenuState {
    #[default]
    Base,
    AttackChoice,
}

#[derive(Default, Debug)]
enum TurnState {
    #[default]
    PlayerTurn,
    EnemyTurn,
}

#[derive(GodotClass)]
#[class(init, base=Node2D)]
struct BattleCoordinator {
    #[export]
    player_beast: Option<Gd<Node>>,
    #[export]
    enemy_beast: Option<Gd<Node>>,
    #[export]
    menu_base: Option<Gd<Control>>,
    menu_state: MenuState,
    turn_state: TurnState,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for BattleCoordinator {
    fn ready(&mut self) {
        godot_print!("Ready");
        self.populate_menu();
    }
    /*
    fn enter_tree(&mut self) {
        godot_print!("Entered Tree");
        self.populate_menu();
    }
    */
}

#[godot_api]
impl BattleCoordinator {
    #[func]
    fn on_attack_pressed(&mut self) {
        self.menu_state = MenuState::AttackChoice;
        self.populate_menu();
    }

    #[func]
    fn on_flee_pressed(&mut self) {
        godot_print!("Fleeing");
    }

    #[func]
    fn on_attack_option_pressed(&mut self, attack_resource: Gd<BeastAttack>) {
        self.handle_attack(BeastOwner::Player, BeastOwner::Enemy, attack_resource);
    }
}

impl BattleCoordinator {
    fn populate_menu(&mut self) {
        godot_print!("Populating menu");
        let base: Gd<Node2D> = (*self.base()).clone();
        let menu_base = self.menu_base.as_mut().unwrap();
        // Clear existing
        for mut child in menu_base.get_children().iter_shared() {
            child.queue_free();
        }

        match self.turn_state {
            TurnState::PlayerTurn => match self.menu_state {
                MenuState::Base => {
                    let mut attack_button = Button::new_alloc();
                    attack_button.set_text("Attack");
                    menu_base.add_child(&attack_button);
                    attack_button.connect("pressed", &base.callable("on_attack_pressed"));
                    let mut flee_button = Button::new_alloc();
                    flee_button.set_text("Flee");
                    menu_base.add_child(&flee_button);
                    flee_button.connect("pressed", &base.callable("on_flee_pressed"));
                }
                MenuState::AttackChoice => {
                    let attacks = self
                        .player_beast
                        .as_ref()
                        .unwrap()
                        .clone()
                        .cast::<Beast>()
                        .bind()
                        .attacks
                        .clone();
                    for (_i, attack) in attacks.iter_shared().enumerate() {
                        let mut attack_button = Button::new_alloc();
                        attack_button.set_text(&attack.bind().name);
                        menu_base.add_child(&attack_button);
                        attack_button.connect(
                            "pressed",
                            &base
                                .callable(&format!("on_attack_option_pressed"))
                                .bindv(&array![&attack.clone().to_variant()]),
                        );
                    }
                }
            },
            TurnState::EnemyTurn => {
                godot_print!("Enemy's turn");
            }
        }
    }

    fn handle_attack(
        &mut self,
        attacker: BeastOwner,
        attackee: BeastOwner,
        attack_resource: Gd<BeastAttack>,
    ) {
        godot_print!(
            "Handling attack {:?}->{:?} ({})",
            attacker,
            attackee,
            attack_resource.bind().name
        );
        let target_beast = match attackee {
            BeastOwner::Player => self.player_beast.as_mut(),
            BeastOwner::Enemy => self.enemy_beast.as_mut(),
        };
        if let Some(target_beast) = target_beast {
            let mut target_beast = target_beast.clone().cast::<Beast>();

            let target_beast_type: BeastType = target_beast.bind().type_;
            let type_modifier =
                target_beast_type.modifier_when_receiving(attack_resource.bind().type_);
            // TODO: Include level modifiers
            let damage = attack_resource.bind().strength * type_modifier;
            godot_print!(
                "Dealing {} damage from {:?} to {:?}",
                damage,
                attacker,
                attackee
            );
            target_beast.bind_mut().change_health(-1.0 * damage);
        }
        self.menu_state = MenuState::Base;
        self.populate_menu();
    }
}

#[derive(GodotConvert, Var, Export, Default, Debug, Clone, Copy)]
#[godot(via = GString)]
pub enum BeastType {
    #[default]
    Basic,
    Earth,
    Wind,
    Fire,
    Water,
}

impl BeastType {
    fn modifier_when_receiving(&self, attacker: BeastType) -> f64 {
        match (self, attacker) {
            (_, BeastType::Basic) => 0.9,
            (BeastType::Earth, BeastType::Wind) => 0.5,
            (BeastType::Earth, BeastType::Fire) => 2.0,
            (BeastType::Wind, BeastType::Fire) => 0.5,
            (BeastType::Wind, BeastType::Earth) => 2.0,
            (BeastType::Fire, BeastType::Earth) => 0.5,
            (BeastType::Fire, BeastType::Wind) => 2.0,
            _ => 1.0,
        }
    }
}

#[derive(GodotClass)]
#[class(init, base=Node2D)]
struct Beast {
    #[export]
    max_health: f64,
    #[export]
    type_: BeastType,
    #[export]
    name: GString,
    #[export]
    attacks: Array<Gd<BeastAttack>>,
    health: f64,
    base: Base<Node2D>,
}

#[godot_api]
impl INode2D for Beast {
    fn ready(&mut self) {
        self.health = self.max_health;
        self.on_change_health();
        let mut nametag = self
            .base_mut()
            .find_child("NameTag")
            .unwrap()
            .cast::<godot::classes::Label>();
        nametag.set_text(&self.name);
    }
}

#[godot_api]
impl Beast {
    fn change_health(&mut self, amount: f64) {
        godot_print!("Changing health by {}", amount);
        self.health += amount;
        if self.health > self.max_health {
            self.health = self.max_health;
        }
        if self.health <= 0.0 {
            // TODO: Handle death
            self.base_mut().queue_free();
            self.base_mut().emit_signal("died", &[]);
        }
        self.on_change_health();
    }

    #[signal]
    fn died(&self);
}

impl Beast {
    fn on_change_health(&mut self) {
        let mut bar = self.base_mut().find_child("HealthBar").unwrap();
        bar.set("value", &Variant::from(self.health));
        bar.set("max_value", &Variant::from(self.max_health));
    }
}

#[derive(GodotClass)]
#[class(tool, init, base=Resource)]
struct BeastAttack {
    #[export]
    name: GString,
    #[export]
    type_: BeastType,
    #[export]
    strength: f64,
    base: Base<Resource>,
}
