use std::collections::{HashMap, HashSet, VecDeque};

use daggy::{NodeIndex, Walker};

use crate::{graph::Adag, log};

use self::bisection_tree::*;

use super::{RegressionAlgorithm, RegressionPoint, TestResult};

pub struct GitBisect {
    graph: Adag<String, ()>,
    results: HashMap<NodeIndex, TestResult>,
    valid_nodes: HashSet<NodeIndex>,
    ignored_nodes: HashSet<NodeIndex>,
    bisection_tree: BisectionTree,
    jobs: VecDeque<NodeIndex>,
    jobs_await: HashSet<NodeIndex>,
    interrupts: HashSet<String>,
    original_target: NodeIndex,
    current_target: NodeIndex,
    log_path: Option<std::path::PathBuf>,
}

impl GitBisect {
    pub fn new(graph: Adag<String, ()>, log_path: Option<std::path::PathBuf>) -> Self {
        let mut sources_index = HashSet::from_iter(
            graph
                .sources
                .iter()
                .filter_map(|hash| graph.indexation.get(hash))
                .copied(),
        );

        let mut target_index = graph.index(&graph.targets[0]);
        let mut ignored_nodes = HashSet::new();
        let results = HashMap::new();
        let tree = new_root(
            &graph,
            &results,
            &mut sources_index,
            &mut ignored_nodes,
            &mut target_index,
        );

        eprintln!(
            "----\nBisect initialized\n{} Commits\n----",
            graph.graph.node_count()
        );

        let bisect_log_dir = log_path.map(|p| log::add_dir("bisect", &p));
        if let Some(log_dir) = &bisect_log_dir {
            log::create_file("summary", log_dir);
            log::write_to_file(
                &format!(
                    "Init Bisect:\nSpeculation Tree:\n{}\n---\n\n",
                    tree.display(&graph, &results)
                ),
                &summary_log_file(log_dir),
            )

        }

        GitBisect {
            graph,
            valid_nodes: sources_index,
            ignored_nodes,
            results,
            original_target: target_index,
            bisection_tree: tree,
            jobs_await: HashSet::new(),
            jobs: VecDeque::new(),
            interrupts: HashSet::new(),
            current_target: target_index,
            log_path: bisect_log_dir,
        }
    }

    fn extend_speculation_tree(&mut self) -> bool {
        let mut changed = false;
        match self.bisection_tree {
            Child::End => {}

            Child::Unknown => {
                changed = true;
                self.bisection_tree = new_root(
                    &self.graph,
                    &self.results,
                    &mut self.valid_nodes,
                    &mut self.ignored_nodes,
                    &mut self.current_target,
                );
            }

            Child::Next(ref mut root) => {
                let mut q =
                    VecDeque::<(&mut Box<Node>, usize, HashSet<NodeIndex>, NodeIndex)>::new();
                let mut limit = None;
                q.push_back((root, 0, HashSet::new(), self.current_target));

                while let Some((node, depth, mut vs, t)) = q.pop_front() {
                    if let Some(lim) = limit {
                        if lim < depth {
                            break;
                        }
                    }

                    let result = self.results.get(&node.index);

                    if let Some(TestResult::True) | Some(TestResult::Ignore) | None = result {
                        let mut vs2 = vs.clone();
                        if let Some(TestResult::True) | None = result {
                            vs2.insert(node.index);
                        }

                        match node.right {
                            Child::Next(ref mut r) => q.push_back((r, depth + 1, vs2, t)),

                            ref mut right @ Child::Unknown => {
                                limit.get_or_insert(depth);

                                vs2.insert(node.index); //second time for the ignore case.
                                vs2.extend(self.valid_nodes.iter());
                                let bisect = associated_value_bisection(
                                    &self.graph,
                                    &vs2,
                                    &self.ignored_nodes,
                                    t,
                                );

                                changed = true;
                                *right = bisect.into();
                            }

                            Child::End => {}
                        }
                    }

                    if let Some(TestResult::False) | None = result {
                        match node.left {
                            Child::Next(ref mut l) => {
                                q.push_back((l, depth + 1, vs.clone(), node.index))
                            }

                            ref mut left @ Child::Unknown => {
                                limit.get_or_insert(depth);

                                vs.extend(self.valid_nodes.iter());
                                let bisect = associated_value_bisection(
                                    &self.graph,
                                    &vs,
                                    &self.ignored_nodes,
                                    node.index,
                                );

                                changed = true;
                                *left = bisect.into();
                            }

                            Child::End => {}
                        }
                    }
                }
            }
        }
        changed
    }

    fn extract_jobs(&self) -> HashSet<NodeIndex> {
        if let Child::Next(root) = &self.bisection_tree {
            let mut jobs = HashSet::new();
            let mut limit = None;
            let mut q: VecDeque<(&Box<Node>, usize)> = VecDeque::new();
            q.push_back((root, 0));

            while let Some((node, depth)) = q.pop_front() {
                if let Some(lim) = limit {
                    if lim < depth {
                        break;
                    }
                }

                if self.is_unprocessed(node.index) {
                    limit.get_or_insert(depth);
                    jobs.insert(node.index);
                    continue;
                }

                let result = self.results.get(&node.index);
                if let Some(TestResult::True) | Some(TestResult::Ignore) | None = result {
                    if let Child::Next(ref r) = node.right {
                        q.push_back((r, depth + 1));
                    }
                }

                if let Some(TestResult::False) | None = result {
                    if let Child::Next(ref l) = node.left {
                        q.push_back((l, depth + 1));
                    }
                }
            }
            jobs
        } else {
            HashSet::default()
        }
    }

    fn is_unprocessed(&self, index: NodeIndex) -> bool {
        !self.results.contains_key(&index) && !self.jobs_await.contains(&index)
    }
}

fn new_root(
    graph: &Adag<String, ()>,
    results: &HashMap<NodeIndex, TestResult>,
    valid_nodes: &mut HashSet<NodeIndex>,
    ignored_nodes: &mut HashSet<NodeIndex>,
    target: &mut NodeIndex,
) -> Child<Node> {
    let mut fictive_valids = valid_nodes.clone(); //fixme: bad! Might not work correctly

    //Given the current results, we can calculate a new root
    //That root might already have a result, so we have to repeat this process until
    //we find a unevaluated root or we know that there is nothing left.
    let root = loop {
        let node = associated_value_bisection(graph, valid_nodes, &fictive_valids, *target);
        match node {
            None => break node,
            Some(n) => match results.get(&n) {
                Some(TestResult::True) => {
                    valid_nodes.insert(n);
                    fictive_valids.insert(n);
                }
                Some(TestResult::False) => {
                    *target = n;
                }
                Some(TestResult::Ignore) => {
                    fictive_valids.insert(n);
                    ignored_nodes.insert(n);
                }
                None => break node,
            },
        }
    };

    root.into()
}

impl RegressionAlgorithm for GitBisect {
    fn add_result(&mut self, commit: String, result: super::TestResult) {
        self.results
            .insert(self.graph.index(&commit), result.clone());
        self.jobs_await.remove(&self.graph.index(&commit));

        //temporarily move bisection_tree into current.
        //Child::Unknown is only a placeholder, we'll override it later again.
        let mut current = std::mem::replace(&mut self.bisection_tree, Child::Unknown);
        let mut changed = false;
        while let Child::Next(ref c) = current {
            match self.results.get(&c.index) {
                Some(TestResult::True) => {
                    self.valid_nodes.insert(c.index);
                    current = current.unwrap().right;
                    changed = true;
                }
                Some(TestResult::False) => {
                    self.current_target = c.index;
                    current = current.unwrap().left;
                    changed = true;
                }
                Some(TestResult::Ignore) => {
                    self.ignored_nodes.insert(c.index);
                    current = current.unwrap().right;
                    changed = true;
                }
                None => break,
            }
        }

        self.bisection_tree = current;

        if changed {
            let (remaining_nodes, _) =
                get_subgraph(&self.graph, &self.valid_nodes, self.current_target);
            self.interrupts.extend(
                self.jobs_await
                    .difference(&remaining_nodes)
                    .map(|i| self.graph.node_from_index(*i)),
            );
        }

        if let Child::Unknown = self.bisection_tree {
            self.bisection_tree = new_root(
                &self.graph,
                &self.results,
                &mut self.valid_nodes,
                &mut self.ignored_nodes,
                &mut self.current_target,
            );
        }

        //Logging
        if let Some(log_path) = &self.log_path {
            let in_progress = self
                .jobs_await
                .iter()
                .map(|i| self.graph.node_from_index(*i))
                .collect::<Vec<_>>();
            let interrupting = self.interrupts.clone();
            log::write_to_file(
                &format!(
                    "Add Result: {} {}\nIn progress: {:?}\nInterrupting: {:?}\nSpeculation Tree:\n{}\n---\n\n",
                    commit, result, in_progress, interrupting, self.bisection_tree.display(&self.graph, &self.results)
                ),
                &summary_log_file(log_path),
            )
        }
        //
    }

    fn next_job(&mut self, capacity: u32, _expected_capacity: u32) -> super::AlgorithmResponse {
        if self.jobs.is_empty() {
            //First look if there are any jobs left in the tree.
            let mut jobs = VecDeque::from_iter(self.extract_jobs().into_iter());

            //Otherwise try to extend the tree and collect the new jobs
            while jobs.is_empty() {
                let changed = self.extend_speculation_tree();
                jobs = VecDeque::from_iter(self.extract_jobs().into_iter());

                if !changed {
                    break;
                }
            }

            if (capacity as usize) >= jobs.len() {
                self.jobs = jobs;
            }
        }

        match (self.jobs.pop_front(), self.jobs_await.is_empty()) {
            (Some(job), _) => {
                //Logging
                if let Some(log_path) = &self.log_path {
                    let in_progress = self
                        .jobs_await
                        .iter()
                        .map(|i| self.graph.node_from_index(*i))
                        .collect::<Vec<_>>();
                    let interrupting = self.interrupts.clone();
                    log::write_to_file(
                        &format!(
                            "New Job: {} for capacity {}\nIn progress: {:?}\nInterrupting: {:?}\nSpeculation Tree:\n{}\n---\n\n",
                            self.graph.node_from_index(job), capacity, in_progress, interrupting, self.bisection_tree.display(&self.graph, &self.results)
                        ),
                        &summary_log_file(log_path),
                    )
                }
                //

                self.jobs_await.insert(job);
                let hash = self.graph.node_from_index(job);
                super::AlgorithmResponse::Job(hash)
            }
            (None, false) => super::AlgorithmResponse::WaitForResult,
            (None, true) => super::AlgorithmResponse::InternalError("Bisect: Search error!"),
        }
    }

    fn interrupts(&mut self) -> Vec<String> {
        let res = std::mem::take(&mut self.interrupts);
        Vec::from_iter(res.into_iter())
    }

    fn done(&self) -> bool {
        matches!(self.bisection_tree, Child::End)
    }

    fn results(&self) -> Vec<super::RegressionPoint> {
        vec![RegressionPoint {
            target: self.graph.node_from_index(self.original_target),
            regression_point: self.graph.node_from_index(self.current_target),
        }]
    }
}

pub fn summary_log_file(path: &std::path::Path) -> std::path::PathBuf {
    path.join("summary")
}

pub fn associated_value_bisection(
    graph: &Adag<String, ()>,
    sources: &HashSet<NodeIndex>,
    ignored: &HashSet<NodeIndex>,
    target: NodeIndex,
) -> Option<NodeIndex> {
    let (relevant, start_points) = get_subgraph(graph, sources, target);

    //starting from the relevant sources, we're now calculating the number of parents for all nodes
    let mut number_of_parents = HashMap::<NodeIndex, usize>::new();
    let mut queue = VecDeque::<NodeIndex>::new();
    let mut size = start_points.len();

    for source in &start_points {
        queue.push_back(*source);
        number_of_parents.insert(*source, 0);
    }

    while let Some(current_index) = queue.pop_front() {
        if current_index == target {
            continue;
        }

        for (_, child_index) in graph.graph.children(current_index).iter(&graph.graph) {
            if relevant.contains(&child_index) {
                match number_of_parents.get_mut(&child_index) {
                    Some(np) => {
                        *np += 1;
                    }
                    None => {
                        number_of_parents.insert(child_index, 1);
                        queue.push_back(child_index);
                        size += 1;
                    }
                };
            }
        }
    }
    drop(queue);

    if size <= 1 {
        return None;
    }
    //calculating the number of ancestors, by performing a topological sort
    //we keep track of simple and disjunct sets of nodes in the graph.
    //For each of such a section we store the size of it in an hashmap
    //The number of ancestors for a node, is the size of all sets that have ancestors of that node
    //  + the number of ancestors in the set of the active nodes.
    let mut section_sizes = HashMap::<NodeIndex, usize>::new();
    let mut sections_of_nodes = HashMap::<NodeIndex, (HashSet<NodeIndex>, usize)>::new();
    let mut number_of_ancestors = HashMap::<NodeIndex, usize>::new();
    let mut queue = VecDeque::<NodeIndex>::new();

    for source in &start_points {
        queue.push_back(*source);
        sections_of_nodes.insert(*source, (HashSet::new(), 1));
    }

    while let Some(current_index) = queue.pop_front() {
        let mut anc = 0;
        let (sections, offset) = sections_of_nodes
            .remove(&current_index)
            .unwrap_or((HashSet::new(), 1));

        anc += offset;
        anc += sections
            .iter()
            .filter_map(|section| section_sizes.get(section))
            .sum::<usize>();
        number_of_ancestors.insert(current_index, anc);

        if current_index == target {
            break;
        }

        if anc > size / 2 {
            continue;
        }

        let children: Vec<NodeIndex> = graph
            .graph
            .children(current_index)
            .iter(&graph.graph)
            .map(|(_, n)| n)
            .filter(|n| relevant.contains(n))
            .collect();

        match children.len().cmp(&1) {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => {
                let child_index = children[0];
                match sections_of_nodes.get_mut(&child_index) {
                    Some((child_section, child_offset)) => {
                        *child_offset += offset;
                        child_section.extend(sections.into_iter());
                    }
                    None => {
                        sections_of_nodes.insert(child_index, (sections, offset + 1));
                    }
                }
            }
            std::cmp::Ordering::Greater => {
                section_sizes.insert(current_index, offset);
                let mut new_sections = sections.clone();
                new_sections.insert(current_index);
                for child_index in &children {
                    match sections_of_nodes.get_mut(child_index) {
                        Some((child_section, _)) => {
                            child_section.extend(new_sections.iter());
                        }
                        None => {
                            sections_of_nodes.insert(*child_index, (new_sections.clone(), 1));
                        }
                    }
                }
            }
        };

        for child_index in children {
            if let Some(parents) = number_of_parents.get_mut(&child_index) {
                *parents -= 1;
                if *parents == 0 {
                    queue.push_back(child_index);
                }
            }
        }
    }

    let (bisection_point, _associated_value) = number_of_ancestors
        .into_iter()
        .filter(|(n, _)| !ignored.contains(n))
        .map(|(n, v)| (n, std::cmp::min(v, size - v)))
        .max_by(|(_, v1), (_, v2)| v1.cmp(v2))
        .unwrap();

    Some(bisection_point)
}

fn get_subgraph(
    graph: &Adag<String, ()>,
    sources: &HashSet<NodeIndex>,
    target: NodeIndex,
) -> (HashSet<NodeIndex>, HashSet<NodeIndex>) {
    // find all nodes reachable from the target
    // let mut relevant_sources = HashSet::<NodeIndex>::new();

    let mut relevant = HashSet::<NodeIndex>::new();
    let mut queue = VecDeque::<NodeIndex>::new();
    let mut irrelevant_queue = VecDeque::<NodeIndex>::new();
    let mut irrelevant = HashSet::<NodeIndex>::new();
    queue.push_back(target);
    relevant.insert(target);

    while let Some(current_index) = queue.pop_front() {
        if sources.contains(&current_index) {
            if irrelevant.insert(current_index) {
                relevant.remove(&current_index);
                irrelevant_queue.push_back(current_index);
            }
            continue;
        }

        for (_, parent_index) in graph.graph.parents(current_index).iter(&graph.graph) {
            if relevant.insert(parent_index) {
                queue.push_back(parent_index);
            }
        }
    }
    drop(queue);

    while let Some(current_index) = irrelevant_queue.pop_front() {
        for (_, parent_index) in graph.graph.parents(current_index).iter(&graph.graph) {
            if irrelevant.insert(parent_index) {
                relevant.remove(&parent_index);
                irrelevant_queue.push_back(parent_index);
            }
        }
    }
    drop(irrelevant_queue);
    drop(irrelevant);

    let mut start_points = HashSet::<NodeIndex>::new();
    let mut visited = HashSet::<NodeIndex>::new();
    let mut queue = VecDeque::<NodeIndex>::new();
    queue.push_back(target);
    visited.insert(target);

    while let Some(current_index) = queue.pop_front() {
        let parents = graph
            .graph
            .parents(current_index)
            .iter(&graph.graph)
            .filter(|(_, parent_index)| relevant.contains(parent_index))
            .collect::<Vec<_>>();

        if parents.is_empty() {
            start_points.insert(current_index);
        } else {
            for (_, parent_index) in parents {
                if visited.insert(parent_index) {
                    queue.push_back(parent_index);
                }
            }
        }
    }

    (relevant, start_points)
}

mod bisection_tree {
    use std::collections::HashMap;

    use daggy::NodeIndex;

    use crate::{graph::Adag, regression::TestResult};
    pub(crate) type BisectionTree = Child<Node>;

    pub(crate) struct Node {
        pub index: NodeIndex,
        pub left: Child<Node>,
        pub right: Child<Node>,
    }
    pub(crate) enum Child<T> {
        Next(Box<T>),
        Unknown,
        End,
    }

    impl<T> Child<T> {
        pub(crate) fn unwrap(self) -> Box<T> {
            match self {
                Child::Next(val) => val,
                _ => {
                    panic!("called `Child::unwrap()` on a empty value");
                }
            }
        }
    }

    impl Node {
        fn new(index: NodeIndex) -> Self {
            Node {
                index,
                left: Child::Unknown,
                right: Child::Unknown,
            }
        }
    }

    impl From<Node> for Child<Node> {
        fn from(node: Node) -> Self {
            Child::Next(Box::new(node))
        }
    }

    impl From<Option<NodeIndex>> for Child<Node> {
        fn from(op: Option<NodeIndex>) -> Self {
            match op {
                Some(index) => Node::new(index).into(),
                None => Child::End,
            }
        }
    }

    impl BisectionTree {
        pub(crate) fn display(&self, context: &Adag<String, ()>, results: &HashMap<NodeIndex, TestResult>) -> String {
            let mut out = String::from("");
            let mut stack = Vec::<_>::new();
            stack.push((self, 0, None));

            while let Some((current, level, dir)) = stack.pop() {
                let a = "   |".repeat(level);
                let b = match dir {
                    Some(true) => "+++",
                    Some(false) => "---",
                    None => "",
                };

                let c = match current {
                    Child::Next(n) => {
                        stack.push((&n.left, level + 1, Some(false)));
                        stack.push((&n.right, level + 1, Some(true)));
                        let hash = context.node_from_index(n.index);
                        match results.get(&n.index) {
                            Some(res) => format!("{} ({})", hash, res),
                            None => hash,
                        }
                        
                    }
                    Child::Unknown => String::from("unknown"),
                    Child::End => String::from("end"),
                };

                out.push_str(&format!("{}{} {}\n", a, b, c));
            }

            out
        }
    }
}
