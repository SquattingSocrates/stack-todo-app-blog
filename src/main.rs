use std::collections::{HashMap, VecDeque};

use lunatic::{
    abstract_process,
    ap::{Config, ProcessRef},
    AbstractProcess, Tag,
};
use serde::{Deserialize, Serialize};
use submillisecond::{router, Application, Json, Router};

// =====================================
// DTOs
// =====================================
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Task {
    id: u32,
    title: String,
    description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PileInfo {
    id: u32,
    name: String,
    description: String,
    is_stack: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreatePileDTO {
    name: String,
    description: String,
    is_stack: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Pile {
    info: PileInfo,
    tasks: VecDeque<Task>,
}

#[abstract_process(visibility = pub)]
impl Pile {
    #[init]
    fn init(_: Config<Self>, info: PileInfo) -> Result<Self, ()> {
        Ok(Self {
            info,
            tasks: VecDeque::new(),
        })
    }

    #[terminate]
    fn terminate(self) {
        println!("Shutdown process");
    }

    #[handle_link_death]
    fn handle_link_death(&self, _tag: Tag) {
        println!("Link trapped");
    }

    #[handle_request]
    fn complete_current(&mut self) -> Option<Task> {
        if self.info.is_stack {
            return self.tasks.pop_back();
        }
        self.tasks.pop_front()
    }

    #[handle_request]
    fn push_task(&mut self, new_task: Task) -> () {
        self.tasks.push_back(new_task)
    }

    #[handle_request]
    fn pile_top<'a>(&self) -> Option<Task> {
        // we want to ALWAYS give the top element in the stack
        let top = if self.info.is_stack {
            self.tasks.back()
        } else {
            self.tasks.front()
        };
        top.map(|t| t.clone())
    }
}

// a place to register all the piles

#[derive(Debug, Default)]
struct PileRegistry {
    counter: u32,
    piles: HashMap<u32, ProcessRef<Pile>>,
}

#[abstract_process]
impl PileRegistry {
    #[init]
    fn init(_: Config<Self>, _: ()) -> Result<Self, ()> {
        Ok(Self::default())
    }

    #[terminate]
    fn terminate(self) {
        println!("Shutdown process");
    }

    #[handle_link_death]
    fn handle_link_death(&self, _tag: Tag) {
        println!("Link trapped");
    }

    #[handle_request]
    fn create_pile(
        &mut self,
        name: String,
        description: String,
        is_stack: bool,
    ) -> (PileInfo, ProcessRef<Pile>) {
        let id = self.counter;
        self.counter += 1;
        let info = PileInfo {
            id,
            name,
            description,
            is_stack,
        };
        let process_ref = Pile::start(info.clone()).unwrap();
        self.piles.insert(id, process_ref);
        (info, process_ref)
    }

    #[handle_request]
    fn get_pile(&mut self, pile_id: u32) -> Option<ProcessRef<Pile>> {
        self.piles.get(&pile_id).map(|pile| pile.clone())
    }

    #[handle_request]
    fn delete_pile(&mut self, pile_id: u32) -> () {
        if let Some(pile) = self.piles.get(&pile_id) {
            pile.kill();
            self.piles.remove(&pile_id);
        }
    }
}

// =====================================
// Handler functions
// =====================================
fn liveness_check() -> &'static str {
    println!("Running liveness check");
    r#"{"status":"UP"}"#
}

// pile CRUD
fn create_pile(Json(dto): Json<CreatePileDTO>) -> Json<PileInfo> {
    let registry = ProcessRef::<PileRegistry>::lookup(&"registry").unwrap();
    Json(
        registry
            .create_pile(dto.name, dto.description, dto.is_stack)
            .0,
    )
}

// =====================================
// Router and app initialisation
// =====================================
const ROUTER: Router = router! {
    "/api/alive" => liveness_check

    POST "/api/pile" => create_pile
};

fn main() -> std::io::Result<()> {
    let _registry = PileRegistry::start_as(&"registry", ()).expect("should initialize registry");
    Application::new(ROUTER).serve("0.0.0.0:3000")
}
