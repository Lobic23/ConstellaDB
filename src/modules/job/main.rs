use std::fmt;

#[derive(Clone)]
struct QueryJob {
    id: u64,
    query: String,
}

struct Node {
    id: u64,
    active: bool,
    capacity: usize,
    current_load: usize,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Node {} | active={} | load={}/{}",
            self.id,
            self.active,
            self.current_load,
            self.capacity
        )
    }
}

struct Scheduler {
    nodes: Vec<Node>,
    next_node: usize,
}

impl Scheduler {
    fn new(nodes: Vec<Node>) -> Self {
        Self {
            nodes,
            next_node: 0,
        }
    }

    fn schedule_query(&mut self, job: QueryJob) -> Option<u64> {
        let total_nodes = self.nodes.len();

        for _ in 0..total_nodes {
            let idx = self.next_node;

            self.next_node =
                (self.next_node + 1) % total_nodes;

            let node = &mut self.nodes[idx];

            if node.active
                && node.current_load < node.capacity
            {
                node.current_load += 1;

                println!(
                    "\nDispatching Query {}",
                    job.id
                );

                println!(
                    "Query: {}",
                    job.query
                );

                println!(
                    "Assigned to Node {}\n",
                    node.id
                );

                return Some(node.id);
            }
        }

        println!(
            "No available node for Query {}",
            job.id
        );

        None
    }

    fn complete_job(&mut self, node_id: u64) {
        if let Some(node) = self
            .nodes
            .iter_mut()
            .find(|n| n.id == node_id)
        {
            if node.current_load > 0 {
                node.current_load -= 1;
            }
        }
    }

    fn fail_node(&mut self, node_id: u64) {
        if let Some(node) = self
            .nodes
            .iter_mut()
            .find(|n| n.id == node_id)
        {
            node.active = false;

            println!(
                "\nNode {} FAILED\n",
                node_id
            );
        }
    }

    fn recover_node(&mut self, node_id: u64) {
        if let Some(node) = self
            .nodes
            .iter_mut()
            .find(|n| n.id == node_id)
        {
            node.active = true;

            println!(
                "\nNode {} RECOVERED\n",
                node_id
            );
        }
    }

    fn print_cluster_status(&self) {
        println!("\n===== CLUSTER STATUS =====");

        for node in &self.nodes {
            println!("{}", node);
        }

        println!("==========================\n");
    }
}

fn main() {
    let nodes = vec![
        Node {
            id: 1,
            active: true,
            capacity: 2,
            current_load: 0,
        },
        Node {
            id: 2,
            active: true,
            capacity: 2,
            current_load: 0,
        },
        Node {
            id: 3,
            active: true,
            capacity: 2,
            current_load: 0,
        },
    ];

    let mut scheduler =
        Scheduler::new(nodes);

    scheduler.print_cluster_status();

    scheduler.schedule_query(QueryJob {
        id: 1,
        query: "SELECT * FROM users".into(),
    });

    scheduler.schedule_query(QueryJob {
        id: 2,
        query: "SELECT * FROM orders".into(),
    });

    scheduler.schedule_query(QueryJob {
        id: 3,
        query: "SELECT * FROM products".into(),
    });

    scheduler.print_cluster_status();

    scheduler.fail_node(2);

    scheduler.schedule_query(QueryJob {
        id: 4,
        query: "SELECT * FROM payments".into(),
    });

    scheduler.schedule_query(QueryJob {
        id: 5,
        query: "SELECT * FROM logs".into(),
    });

    scheduler.print_cluster_status();

    scheduler.complete_job(1);

    scheduler.recover_node(2);

    scheduler.schedule_query(QueryJob {
        id: 6,
        query: "SELECT * FROM inventory".into(),
    });

    scheduler.print_cluster_status();
}