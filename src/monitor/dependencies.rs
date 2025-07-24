use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use serde::{Deserialize, Serialize};
use crate::monitor::ProcessInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDependencyGraph {
    pub processes: HashMap<u32, ProcessNode>,
    pub shared_libraries: HashMap<String, SharedLibrary>,
    pub dependency_chains: Vec<DependencyChain>,
    pub circular_dependencies: Vec<CircularDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessNode {
    pub pid: u32,
    pub name: String,
    pub ppid: Option<u32>,
    pub children: Vec<u32>,
    pub shared_libs: Vec<String>,
    pub memory_maps: Vec<MemoryMap>,
    pub open_files: Vec<String>,
    pub sockets: Vec<SocketInfo>,
    pub dependency_level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedLibrary {
    pub path: String,
    pub size: u64,
    pub processes_using: Vec<u32>,
    pub version: Option<String>,
    pub is_system_lib: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMap {
    pub start_addr: u64,
    pub end_addr: u64,
    pub permissions: String,
    pub offset: u64,
    pub device: String,
    pub inode: u64,
    pub pathname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketInfo {
    pub local_addr: String,
    pub remote_addr: Option<String>,
    pub state: String,
    pub socket_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyChain {
    pub root_pid: u32,
    pub chain: Vec<u32>,
    pub depth: u32,
    pub shared_resources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularDependency {
    pub processes: Vec<u32>,
    pub dependency_type: String,
    pub resource: String,
}

pub struct DependencyAnalyzer {
    proc_path: String,
}

impl DependencyAnalyzer {
    pub fn new() -> Self {
        Self {
            proc_path: "/proc".to_string(),
        }
    }

    pub fn analyze_dependencies(&self, processes: &[ProcessInfo]) -> Result<ProcessDependencyGraph> {
        let mut graph = ProcessDependencyGraph {
            processes: HashMap::new(),
            shared_libraries: HashMap::new(),
            dependency_chains: Vec::new(),
            circular_dependencies: Vec::new(),
        };

        // Build process nodes
        for process in processes {
            if let Ok(node) = self.build_process_node(process) {
                graph.processes.insert(process.pid, node);
            }
        }

        // Analyze shared libraries
        self.analyze_shared_libraries(&mut graph)?;

        // Build dependency chains
        self.build_dependency_chains(&mut graph);

        // Detect circular dependencies
        self.detect_circular_dependencies(&mut graph);

        // Calculate dependency levels
        self.calculate_dependency_levels(&mut graph);

        Ok(graph)
    }

    fn build_process_node(&self, process: &ProcessInfo) -> Result<ProcessNode> {
        let pid = process.pid;
        let _proc_dir = format!("{}/{}", self.proc_path, pid);
        
        let shared_libs = self.get_shared_libraries(pid)?;
        let memory_maps = self.get_memory_maps(pid)?;
        let open_files = self.get_open_files(pid)?;
        let sockets = self.get_sockets(pid)?;

        // Get children from process list
        let children = Vec::new(); // Will be populated later

        Ok(ProcessNode {
            pid,
            name: process.name.clone(),
            ppid: process.parent_pid,
            children,
            shared_libs,
            memory_maps,
            open_files,
            sockets,
            dependency_level: 0,
        })
    }

    fn get_shared_libraries(&self, pid: u32) -> Result<Vec<String>> {
        let maps_path = format!("{}/{}/maps", self.proc_path, pid);
        let mut libraries = HashSet::new();

        if let Ok(content) = fs::read_to_string(maps_path) {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 6 {
                    let pathname = parts[5];
                    if pathname.ends_with(".so") || pathname.contains(".so.") {
                        libraries.insert(pathname.to_string());
                    }
                }
            }
        }

        Ok(libraries.into_iter().collect())
    }

    fn get_memory_maps(&self, pid: u32) -> Result<Vec<MemoryMap>> {
        let maps_path = format!("{}/{}/maps", self.proc_path, pid);
        let mut maps = Vec::new();

        if let Ok(content) = fs::read_to_string(maps_path) {
            for line in content.lines() {
                if let Ok(map) = self.parse_memory_map_line(line) {
                    maps.push(map);
                }
            }
        }

        Ok(maps)
    }

    fn parse_memory_map_line(&self, line: &str) -> Result<MemoryMap> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            return Err(anyhow::anyhow!("Invalid memory map line"));
        }

        // Parse address range
        let addr_parts: Vec<&str> = parts[0].split('-').collect();
        let start_addr = u64::from_str_radix(addr_parts[0], 16)?;
        let end_addr = u64::from_str_radix(addr_parts[1], 16)?;

        let permissions = parts[1].to_string();
        let offset = u64::from_str_radix(parts[2], 16)?;
        let device = parts[3].to_string();
        let inode = parts[4].parse()?;
        let pathname = if parts.len() > 5 {
            Some(parts[5].to_string())
        } else {
            None
        };

        Ok(MemoryMap {
            start_addr,
            end_addr,
            permissions,
            offset,
            device,
            inode,
            pathname,
        })
    }

    fn get_open_files(&self, pid: u32) -> Result<Vec<String>> {
        let fd_dir = format!("{}/{}/fd", self.proc_path, pid);
        let mut files = Vec::new();

        if let Ok(entries) = fs::read_dir(fd_dir) {
            for entry in entries.flatten() {
                if let Ok(target) = fs::read_link(entry.path()) {
                    if let Some(path_str) = target.to_str() {
                        files.push(path_str.to_string());
                    }
                }
            }
        }

        Ok(files)
    }

    fn get_sockets(&self, pid: u32) -> Result<Vec<SocketInfo>> {
        let mut sockets = Vec::new();
        
        // Read TCP sockets
        sockets.extend(self.parse_socket_file("/proc/net/tcp", pid)?);
        sockets.extend(self.parse_socket_file("/proc/net/tcp6", pid)?);
        
        // Read UDP sockets
        sockets.extend(self.parse_socket_file("/proc/net/udp", pid)?);
        sockets.extend(self.parse_socket_file("/proc/net/udp6", pid)?);

        Ok(sockets)
    }

    fn parse_socket_file(&self, socket_file: &str, target_pid: u32) -> Result<Vec<SocketInfo>> {
        let mut sockets = Vec::new();
        
        if let Ok(content) = fs::read_to_string(socket_file) {
            for line in content.lines().skip(1) { // Skip header
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    // Check if this socket belongs to our process
                    if let Ok(inode) = parts[9].parse::<u32>() {
                        if self.socket_belongs_to_process(target_pid, inode)? {
                            let local_addr = self.parse_address(parts[1])?;
                            let remote_addr = if parts[2] != "00000000:0000" {
                                Some(self.parse_address(parts[2])?)
                            } else {
                                None
                            };
                            let state = self.parse_socket_state(parts[3])?;
                            let socket_type = if socket_file.contains("tcp") {
                                "TCP".to_string()
                            } else {
                                "UDP".to_string()
                            };

                            sockets.push(SocketInfo {
                                local_addr,
                                remote_addr,
                                state,
                                socket_type,
                            });
                        }
                    }
                }
            }
        }

        Ok(sockets)
    }

    fn socket_belongs_to_process(&self, pid: u32, inode: u32) -> Result<bool> {
        let fd_dir = format!("{}/{}/fd", self.proc_path, pid);
        
        if let Ok(entries) = fs::read_dir(fd_dir) {
            for entry in entries.flatten() {
                if let Ok(target) = fs::read_link(entry.path()) {
                    if let Some(target_str) = target.to_str() {
                        if target_str.starts_with("socket:[") && target_str.contains(&inode.to_string()) {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    fn parse_address(&self, addr_str: &str) -> Result<String> {
        let parts: Vec<&str> = addr_str.split(':').collect();
        if parts.len() != 2 {
            return Ok(addr_str.to_string());
        }

        let ip_hex = parts[0];
        let port_hex = parts[1];

        // Parse IP address (little-endian)
        if ip_hex.len() == 8 {
            // IPv4
            let ip_num = u32::from_str_radix(ip_hex, 16)?;
            let ip = format!("{}.{}.{}.{}", 
                ip_num & 0xFF,
                (ip_num >> 8) & 0xFF,
                (ip_num >> 16) & 0xFF,
                (ip_num >> 24) & 0xFF
            );
            let port = u16::from_str_radix(port_hex, 16)?;
            Ok(format!("{}:{}", ip, port))
        } else {
            // IPv6 or other format
            Ok(addr_str.to_string())
        }
    }

    fn parse_socket_state(&self, state_hex: &str) -> Result<String> {
        let state_num = u8::from_str_radix(state_hex, 16)?;
        let state_name = match state_num {
            1 => "ESTABLISHED",
            2 => "SYN_SENT",
            3 => "SYN_RECV",
            4 => "FIN_WAIT1",
            5 => "FIN_WAIT2",
            6 => "TIME_WAIT",
            7 => "CLOSE",
            8 => "CLOSE_WAIT",
            9 => "LAST_ACK",
            10 => "LISTEN",
            11 => "CLOSING",
            _ => "UNKNOWN",
        };
        Ok(state_name.to_string())
    }

    fn analyze_shared_libraries(&self, graph: &mut ProcessDependencyGraph) -> Result<()> {
        let mut lib_usage: HashMap<String, Vec<u32>> = HashMap::new();

        // Collect all shared libraries and their users
        for (pid, node) in &graph.processes {
            for lib in &node.shared_libs {
                lib_usage.entry(lib.clone()).or_default().push(*pid);
            }
        }

        // Create SharedLibrary entries
        for (lib_path, pids) in lib_usage {
            let mut size = 0;
            let mut version = None;
            let is_system_lib = lib_path.starts_with("/lib") || 
                               lib_path.starts_with("/usr/lib") ||
                               lib_path.starts_with("/lib64");

            // Try to get library size
            if let Ok(metadata) = fs::metadata(&lib_path) {
                size = metadata.len();
            }

            // Try to extract version from path
            if let Some(so_pos) = lib_path.find(".so.") {
                version = Some(lib_path[so_pos + 4..].to_string());
            }

            graph.shared_libraries.insert(lib_path.clone(), SharedLibrary {
                path: lib_path,
                size,
                processes_using: pids,
                version,
                is_system_lib,
            });
        }

        Ok(())
    }

    fn build_dependency_chains(&self, graph: &mut ProcessDependencyGraph) {
        let mut visited = HashSet::new();
        
        for (pid, _) in &graph.processes {
            if !visited.contains(pid) {
                if let Some(chain) = self.build_chain_from_root(*pid, graph, &mut visited) {
                    graph.dependency_chains.push(chain);
                }
            }
        }
    }

    fn build_chain_from_root(&self, root_pid: u32, graph: &ProcessDependencyGraph, visited: &mut HashSet<u32>) -> Option<DependencyChain> {
        let mut chain = Vec::new();
        let mut queue = VecDeque::new();
        let mut local_visited = HashSet::new();
        let mut shared_resources = HashSet::new();

        queue.push_back(root_pid);

        while let Some(pid) = queue.pop_front() {
            if local_visited.contains(&pid) {
                continue;
            }
            
            local_visited.insert(pid);
            visited.insert(pid);
            chain.push(pid);

            if let Some(node) = graph.processes.get(&pid) {
                // Add shared libraries as shared resources
                for lib in &node.shared_libs {
                    shared_resources.insert(lib.clone());
                }

                // Add child processes to queue
                for &child_pid in &node.children {
                    if !local_visited.contains(&child_pid) {
                        queue.push_back(child_pid);
                    }
                }
            }
        }

        if chain.len() > 1 {
            Some(DependencyChain {
                root_pid,
                chain,
                depth: local_visited.len() as u32,
                shared_resources: shared_resources.into_iter().collect(),
            })
        } else {
            None
        }
    }

    fn detect_circular_dependencies(&self, graph: &mut ProcessDependencyGraph) {
        // Check for circular dependencies in shared libraries
        for (lib_path, lib_info) in &graph.shared_libraries {
            if lib_info.processes_using.len() > 1 {
                // Look for processes that might have circular dependency through this library
                for i in 0..lib_info.processes_using.len() {
                    for j in (i + 1)..lib_info.processes_using.len() {
                        let pid1 = lib_info.processes_using[i];
                        let pid2 = lib_info.processes_using[j];
                        
                        if self.processes_have_circular_dependency(pid1, pid2, graph) {
                            graph.circular_dependencies.push(CircularDependency {
                                processes: vec![pid1, pid2],
                                dependency_type: "SharedLibrary".to_string(),
                                resource: lib_path.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    fn processes_have_circular_dependency(&self, pid1: u32, pid2: u32, graph: &ProcessDependencyGraph) -> bool {
        // Simple check: if processes share multiple resources, consider it circular
        if let (Some(node1), Some(node2)) = (graph.processes.get(&pid1), graph.processes.get(&pid2)) {
            let shared_libs1: HashSet<_> = node1.shared_libs.iter().collect();
            let shared_libs2: HashSet<_> = node2.shared_libs.iter().collect();
            let intersection: Vec<_> = shared_libs1.intersection(&shared_libs2).collect();
            
            intersection.len() > 2 // More than 2 shared libraries indicates potential circular dependency
        } else {
            false
        }
    }

    fn calculate_dependency_levels(&self, graph: &mut ProcessDependencyGraph) {
        for chain in &graph.dependency_chains {
            for (level, &pid) in chain.chain.iter().enumerate() {
                if let Some(node) = graph.processes.get_mut(&pid) {
                    node.dependency_level = level as u32;
                }
            }
        }
    }

    pub fn get_process_dependencies(&self, pid: u32, graph: &ProcessDependencyGraph) -> Option<Vec<u32>> {
        graph.processes.get(&pid).map(|node| {
            let mut deps = node.children.clone();
            
            // Add processes that share libraries
            for lib in &node.shared_libs {
                if let Some(lib_info) = graph.shared_libraries.get(lib) {
                    for &other_pid in &lib_info.processes_using {
                        if other_pid != pid && !deps.contains(&other_pid) {
                            deps.push(other_pid);
                        }
                    }
                }
            }
            
            deps
        })
    }

    pub fn get_dependency_impact(&self, pid: u32, graph: &ProcessDependencyGraph) -> u32 {
        // Calculate how many processes would be affected if this process dies
        self.get_process_dependencies(pid, graph)
            .map(|deps| deps.len() as u32)
            .unwrap_or(0)
    }
}