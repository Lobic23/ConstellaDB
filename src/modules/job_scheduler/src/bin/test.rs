use std::collections::VecDeque;

#[derive(Clone, Debug)]
struct Job {
    id: u32,
    required_cpu: u32,
    remaining_time: u32,
}

#[derive(Debug)]
struct WorkerNode {
    id: usize,
    cpu_capacity: u32,
    used_cpu: u32,
    active: bool,
    current_job: Option<Job>,
}

impl WorkerNode {
    fn can_accept(&self, job: &Job) -> bool {
        self.active
            && self.current_job.is_none()
            && self.used_cpu + job.required_cpu <= self.cpu_capacity
    }
}

struct Scheduler {
    quantum: u32,

    pending: VecDeque<Job>,
    completed: Vec<Job>,

    workers: Vec<WorkerNode>,
}

impl Scheduler {
    fn new(quantum: u32, workers: Vec<WorkerNode>) -> Self {
        Self {
            quantum,
            pending: VecDeque::new(),
            completed: Vec::new(),
            workers,
        }
    }

    fn submit_job(&mut self, job: Job) {
        self.pending.push_back(job);
    }

    fn fail_node(&mut self, node_id: usize) {
        for node in &mut self.workers {
            if node.id == node_id {
                node.active = false;

                println!("NODE {} FAILED", node_id);

                if let Some(job) = node.current_job.take() {
                    println!(
                        "Recovering Job {} back to queue",
                        job.id
                    );

                    self.pending.push_back(job);
                }

                node.used_cpu = 0;
            }
        }
    }

    fn schedule_tick(&mut self) {
        for node in &mut self.workers {
            if !node.active {
                continue;
            }

            if node.current_job.is_none() {
                let pos = self.pending.iter()
                    .position(|j| node.can_accept(j));

                if let Some(idx) = pos {
                    let job = self.pending.remove(idx).unwrap();

                    node.used_cpu += job.required_cpu;
                    node.current_job = Some(job);
                }
            }

            if let Some(mut job) = node.current_job.take() {

                let run_time =
                    self.quantum.min(job.remaining_time);

                job.remaining_time -= run_time;

                println!(
                    "Node {} ran Job {} for {} ticks",
                    node.id,
                    job.id,
                    run_time
                );

                if job.remaining_time == 0 {

                    println!(
                        "Job {} COMPLETED",
                        job.id
                    );

                    node.used_cpu -= job.required_cpu;

                    self.completed.push(job);
                } else {

                    println!(
                        "Job {} remaining {}",
                        job.id,
                        job.remaining_time
                    );

                    node.used_cpu -= job.required_cpu;

                    self.pending.push_back(job);
                }
            }
        }
    }

    fn print_status(&self) {
        println!("\n=== STATUS ===");

        println!("Pending: {}", self.pending.len());

        println!(
            "Completed: {}",
            self.completed.len()
        );

        for n in &self.workers {
            println!(
                "Node {} Active={} CPU={}/{}",
                n.id,
                n.active,
                n.used_cpu,
                n.cpu_capacity
            );
        }

        println!("==============\n");
    }
}

fn main() {

    let workers = vec![
        WorkerNode {
            id: 1,
            cpu_capacity: 4,
            used_cpu: 0,
            active: true,
            current_job: None,
        },
        WorkerNode {
            id: 2,
            cpu_capacity: 6,
            used_cpu: 0,
            active: true,
            current_job: None,
        },
    ];

    let mut scheduler =
        Scheduler::new(2, workers);

    scheduler.submit_job(Job {
        id: 1,
        required_cpu: 2,
        remaining_time: 8,
    });

    scheduler.submit_job(Job {
        id: 2,
        required_cpu: 3,
        remaining_time: 5,
    });

    scheduler.submit_job(Job {
        id: 3,
        required_cpu: 5,
        remaining_time: 7,
    });

    for tick in 1..10 {

        println!("\n===== TICK {} =====", tick);

        if tick == 4 {
            scheduler.fail_node(2);
        }

        scheduler.schedule_tick();
        scheduler.print_status();
    }
}