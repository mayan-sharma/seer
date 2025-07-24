use crate::monitor::ProcessInfo;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ProcessTree {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub status: String,
    pub parent_pid: Option<u32>,
    pub children: Vec<ProcessTree>,
    pub depth: usize,
}

impl ProcessTree {
    pub fn new(process: &ProcessInfo, depth: usize) -> Self {
        Self {
            pid: process.pid,
            name: process.name.clone(),
            cpu_usage: process.cpu_usage,
            memory_usage: process.memory_usage,
            status: process.status.as_str().to_string(),
            parent_pid: process.parent_pid,
            children: Vec::new(),
            depth,
        }
    }

    pub fn add_child(&mut self, child: ProcessTree) {
        self.children.push(child);
    }


    pub fn flatten_to_display(&self, result: &mut Vec<ProcessTreeDisplay>) {
        let indent = "  ".repeat(self.depth);
        let prefix = if self.depth > 0 {
            format!("{}├─ ", indent)
        } else {
            String::new()
        };

        result.push(ProcessTreeDisplay {
            pid: self.pid,
            name: format!("{}{}", prefix, self.name),
            cpu_usage: self.cpu_usage,
            memory_usage: self.memory_usage,
            status: self.status.clone(),
            depth: self.depth,
            has_children: !self.children.is_empty(),
        });

        for child in &self.children {
            child.flatten_to_display(result);
        }
    }

}

#[derive(Debug, Clone)]
pub struct ProcessTreeDisplay {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub status: String,
    pub depth: usize,
    pub has_children: bool,
}

pub struct ProcessTreeBuilder;

impl ProcessTreeBuilder {
    pub fn build_tree(processes: &[ProcessInfo]) -> Vec<ProcessTree> {
        let mut process_map: HashMap<u32, ProcessInfo> = HashMap::new();
        let mut children_map: HashMap<u32, Vec<u32>> = HashMap::new();
        
        // Build process map and children map
        for process in processes {
            process_map.insert(process.pid, process.clone());
            
            if let Some(parent_pid) = process.parent_pid {
                children_map.entry(parent_pid).or_default().push(process.pid);
            }
        }
        
        // Find root processes (processes without parents or with non-existent parents)
        let mut roots = Vec::new();
        for process in processes {
            if process.parent_pid.is_none() || 
               process.parent_pid.map_or(true, |pid| !process_map.contains_key(&pid)) {
                roots.push(Self::build_subtree(&process_map, &children_map, process.pid, 0));
            }
        }
        
        // Sort roots by CPU usage (descending)
        roots.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
        
        roots
    }
    
    fn build_subtree(
        process_map: &HashMap<u32, ProcessInfo>,
        children_map: &HashMap<u32, Vec<u32>>,
        pid: u32,
        depth: usize,
    ) -> ProcessTree {
        let process = &process_map[&pid];
        let mut tree = ProcessTree::new(process, depth);
        
        if let Some(child_pids) = children_map.get(&pid) {
            for &child_pid in child_pids {
                if process_map.contains_key(&child_pid) {
                    let child_tree = Self::build_subtree(process_map, children_map, child_pid, depth + 1);
                    tree.add_child(child_tree);
                }
            }
        }
        
        tree
    }
    
    pub fn flatten_tree(trees: &[ProcessTree]) -> Vec<ProcessTreeDisplay> {
        let mut result = Vec::new();
        for tree in trees {
            tree.flatten_to_display(&mut result);
        }
        result
    }
    
    pub fn filter_tree(trees: &[ProcessTree], query: &str) -> Vec<ProcessTree> {
        let mut filtered = Vec::new();
        for tree in trees {
            if let Some(filtered_tree) = Self::filter_subtree(tree, query) {
                filtered.push(filtered_tree);
            }
        }
        filtered
    }
    
    fn filter_subtree(tree: &ProcessTree, query: &str) -> Option<ProcessTree> {
        let matches = tree.name.to_lowercase().contains(&query.to_lowercase());
        
        let mut filtered_children = Vec::new();
        for child in &tree.children {
            if let Some(filtered_child) = Self::filter_subtree(child, query) {
                filtered_children.push(filtered_child);
            }
        }
        
        if matches || !filtered_children.is_empty() {
            let mut filtered_tree = tree.clone();
            filtered_tree.children = filtered_children;
            Some(filtered_tree)
        } else {
            None
        }
    }
}