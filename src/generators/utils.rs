use std::{
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
    hash::Hash,
};

pub fn astar<Context, T: Eq + Hash + Clone>(
    start: &T,
    end: &T,
    context: &Context,
    neighbors_of: impl Fn(&T, &Context) -> Vec<(T, i32)>,
    heuristic: impl Fn(&T, &T, &Context) -> i32,
) -> Option<Vec<T>> {
    #[derive(Clone, Copy)]
    struct Node<T> {
        cost: i32,
        value: T,
    }
    impl<T> PartialEq for Node<T> {
        fn eq(&self, other: &Self) -> bool {
            self.cost.eq(&other.cost)
        }
    }
    impl<T> PartialOrd for Node<T> {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            other.cost.partial_cmp(&self.cost)
        }
    }
    impl<T> Ord for Node<T> {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            other.cost.cmp(&self.cost)
        }
    }
    impl<T> Eq for Node<T> {}
    let mut queue: BinaryHeap<Node<T>> = BinaryHeap::new();
    queue.push(Node {
        cost: 0,
        value: start.clone(),
    });
    let mut came_from: HashMap<T, T> = HashMap::new();
    let mut cost_so_far = HashMap::new();
    cost_so_far.insert(start.clone(), 0);
    while let Some(current) = queue.pop() {
        if current.value == *end {
            break;
        }
        let neigbors = neighbors_of(&current.value, context);
        for (next, cost) in neigbors {
            let new_cost = cost_so_far[&current.value] + cost;
            if cost_so_far.contains_key(&next) {
                if cost_so_far[&next] <= new_cost {
                    continue;
                }
            }
            cost_so_far.insert(next.clone(), new_cost);
            let priority = new_cost + heuristic(&end, &next, &context);
            queue.push(Node {
                cost: priority,
                value: next.clone(),
            });
            came_from.insert(next, current.value.clone());
        }
    }
    let mut base = end.clone();
    let mut out = Vec::new();
    out.push(base.clone());
    while let Some(next) = came_from.get(&base) {
        base = next.clone();
        out.push(base.clone());
        if base == *start {
            return Some(out);
        }
    }
    None
}

pub fn distance_table<Context, T: Eq + Hash + Clone>(
    start: &T,
    context: &Context,
    neighbors_of: impl Fn(&T, &Context) -> Vec<(T, i32)>,
) -> HashMap<T, i32> {
    #[derive(Clone, Copy)]
    struct Node<T> {
        cost: i32,
        value: T,
    }
    impl<T> PartialEq for Node<T> {
        fn eq(&self, other: &Self) -> bool {
            self.cost.eq(&other.cost)
        }
    }
    impl<T> PartialOrd for Node<T> {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            other.cost.partial_cmp(&self.cost)
        }
    }
    impl<T> Ord for Node<T> {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            other.cost.cmp(&self.cost)
        }
    }
    impl<T> Eq for Node<T> {}
    let should_debug = false;
    let mut dbg_count = 0;
    let mut queue: BinaryHeap<Node<T>> = BinaryHeap::new();
    queue.push(Node {
        cost: 0,
        value: start.clone(),
    });
    let mut came_from: HashMap<T, T> = HashMap::new();
    let mut cost_so_far = HashMap::new();
    cost_so_far.insert(start.clone(), 0);
    while let Some(current) = queue.pop() {
        dbg_count += 1;
        let neigbors = neighbors_of(&current.value, context);
        for (next, cost) in neigbors {
            let new_cost = cost_so_far[&current.value] + cost;
            if cost_so_far.contains_key(&next) {
                if cost_so_far[&next] <= new_cost {
                    continue;
                }
            }
            cost_so_far.insert(next.clone(), new_cost);
            let priority = new_cost;
            queue.push(Node {
                cost: priority,
                value: next.clone(),
            });
            came_from.insert(next, current.value.clone());
        }
    }
    if should_debug {
        println!("distance table debug_count:{}", dbg_count);
    }
    cost_so_far
}
