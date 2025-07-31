use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::str;
use tokio::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseMetrics {
    pub mysql: Option<MySQLMetrics>,
    pub postgresql: Option<PostgreSQLMetrics>,
    pub mongodb: Option<MongoDBMetrics>,
    pub redis: Option<RedisMetrics>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySQLMetrics {
    pub connections: ConnectionMetrics,
    pub queries: QueryMetrics,
    pub innodb: InnoDBMetrics,
    pub replication: Option<ReplicationMetrics>,
    pub slow_queries: u64,
    pub uptime: u64,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgreSQLMetrics {
    pub connections: ConnectionMetrics,
    pub database_stats: Vec<DatabaseStats>,
    pub locks: LockMetrics,
    pub replication: Option<PostgreSQLReplicationMetrics>,
    pub cache_hit_ratio: f64,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoDBMetrics {
    pub connections: ConnectionMetrics,
    pub operations: OperationMetrics,
    pub memory: MongoMemoryMetrics,
    pub replication: Option<MongoReplicationMetrics>,
    pub sharding: Option<ShardingMetrics>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisMetrics {
    pub connections: ConnectionMetrics,
    pub memory: RedisMemoryMetrics,
    pub keyspace: KeyspaceMetrics,
    pub persistence: PersistenceMetrics,
    pub replication: Option<RedisReplicationMetrics>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    pub current: u32,
    pub max: u32,
    pub total_created: u64,
    pub aborted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetrics {
    pub select: u64,
    pub insert: u64,
    pub update: u64,
    pub delete: u64,
    pub queries_per_second: f64,
    pub avg_query_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnoDBMetrics {
    pub buffer_pool_size: u64,
    pub buffer_pool_pages_free: u64,
    pub buffer_pool_pages_total: u64,
    pub log_waits: u64,
    pub row_lock_waits: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationMetrics {
    pub slave_lag: Option<u64>,
    pub slave_running: bool,
    pub master_host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub name: String,
    pub size: u64,
    pub connections: u32,
    pub transactions_per_second: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockMetrics {
    pub waiting: u32,
    pub granted: u32,
    pub deadlocks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgreSQLReplicationMetrics {
    pub streaming: bool,
    pub lag_bytes: u64,
    pub sync_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetrics {
    pub queries: u64,
    pub inserts: u64,
    pub updates: u64,
    pub deletes: u64,
    pub getmores: u64,
    pub commands: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoMemoryMetrics {
    pub resident: u64,
    pub virtual_mem: u64,
    pub mapped: u64,
    pub cache_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoReplicationMetrics {
    pub is_master: bool,
    pub is_secondary: bool,
    pub replication_lag: Option<u64>,
    pub oplog_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingMetrics {
    pub chunks: u32,
    pub shards: u32,
    pub balancer_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisMemoryMetrics {
    pub used: u64,
    pub peak: u64,
    pub rss: u64,
    pub fragmentation_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyspaceMetrics {
    pub total_keys: u64,
    pub expires: u64,
    pub expired_keys: u64,
    pub evicted_keys: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceMetrics {
    pub rdb_last_save_time: u64,
    pub rdb_changes_since_last_save: u64,
    pub aof_enabled: bool,
    pub aof_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisReplicationMetrics {
    pub role: String,
    pub connected_slaves: u32,
    pub master_repl_offset: u64,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub mysql_enabled: bool,
    pub mysql_host: String,
    pub mysql_port: u16,
    pub mysql_user: String,
    pub mysql_password: String,
    
    pub postgresql_enabled: bool,
    pub postgresql_host: String,
    pub postgresql_port: u16,
    pub postgresql_user: String,
    pub postgresql_password: String,
    pub postgresql_database: String,
    
    pub mongodb_enabled: bool,
    pub mongodb_host: String,
    pub mongodb_port: u16,
    
    pub redis_enabled: bool,
    pub redis_host: String,
    pub redis_port: u16,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            mysql_enabled: false,
            mysql_host: "localhost".to_string(),
            mysql_port: 3306,
            mysql_user: "root".to_string(),
            mysql_password: String::new(),
            
            postgresql_enabled: false,
            postgresql_host: "localhost".to_string(),
            postgresql_port: 5432,
            postgresql_user: "postgres".to_string(),
            postgresql_password: String::new(),
            postgresql_database: "postgres".to_string(),
            
            mongodb_enabled: false,
            mongodb_host: "localhost".to_string(),
            mongodb_port: 27017,
            
            redis_enabled: false,
            redis_host: "localhost".to_string(),
            redis_port: 6379,
        }
    }
}

pub struct DatabaseMonitor {
    config: DatabaseConfig,
    last_metrics: Option<DatabaseMetrics>,
    last_update: Instant,
}

impl DatabaseMonitor {
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            config,
            last_metrics: None,
            last_update: Instant::now(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(DatabaseConfig::default())
    }

    pub async fn update_metrics(&mut self) -> Result<()> {
        let now = Instant::now();
        if now.duration_since(self.last_update) < Duration::from_secs(30) {
            return Ok(());
        }

        let mut metrics = DatabaseMetrics {
            mysql: None,
            postgresql: None,
            mongodb: None,
            redis: None,
            last_updated: Utc::now(),
        };

        if self.config.mysql_enabled {
            metrics.mysql = self.collect_mysql_metrics().await.ok();
        }

        if self.config.postgresql_enabled {
            metrics.postgresql = self.collect_postgresql_metrics().await.ok();
        }

        if self.config.mongodb_enabled {
            metrics.mongodb = self.collect_mongodb_metrics().await.ok();
        }

        if self.config.redis_enabled {
            metrics.redis = self.collect_redis_metrics().await.ok();
        }

        self.last_metrics = Some(metrics);
        self.last_update = now;
        Ok(())
    }

    pub fn get_metrics(&self) -> Option<&DatabaseMetrics> {
        self.last_metrics.as_ref()
    }

    async fn collect_mysql_metrics(&self) -> Result<MySQLMetrics> {
        let output = Command::new("mysql")
            .args(&[
                "-h", &self.config.mysql_host,
                "-P", &self.config.mysql_port.to_string(),
                "-u", &self.config.mysql_user,
                &format!("-p{}", self.config.mysql_password),
                "-e", "SHOW GLOBAL STATUS; SHOW VARIABLES LIKE 'max_connections'; SELECT VERSION();"
            ])
            .output()?;

        let stdout = str::from_utf8(&output.stdout)?;
        self.parse_mysql_output(stdout)
    }

    fn parse_mysql_output(&self, output: &str) -> Result<MySQLMetrics> {
        let mut status_vars = HashMap::new();
        
        for line in output.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() == 2 {
                status_vars.insert(parts[0].to_string(), parts[1].to_string());
            }
        }

        let connections = ConnectionMetrics {
            current: status_vars.get("Threads_connected")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
            max: status_vars.get("max_connections")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
            total_created: status_vars.get("Connections")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
            aborted: status_vars.get("Aborted_connects")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
        };

        let queries = QueryMetrics {
            select: status_vars.get("Com_select")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
            insert: status_vars.get("Com_insert")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
            update: status_vars.get("Com_update")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
            delete: status_vars.get("Com_delete")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
            queries_per_second: status_vars.get("Queries")
                .and_then(|v| v.parse::<f64>().ok())
                .map(|q| {
                    let uptime: f64 = status_vars.get("Uptime")
                        .and_then(|v| v.parse().ok()).unwrap_or(1.0);
                    q / uptime
                }).unwrap_or(0.0),
            avg_query_time: 0.0, // Would need performance_schema for this
        };

        let innodb = InnoDBMetrics {
            buffer_pool_size: status_vars.get("Innodb_buffer_pool_pages_total")
                .and_then(|v| v.parse::<u64>().ok())
                .map(|p| p * 16384).unwrap_or(0), // Assuming 16KB pages
            buffer_pool_pages_free: status_vars.get("Innodb_buffer_pool_pages_free")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
            buffer_pool_pages_total: status_vars.get("Innodb_buffer_pool_pages_total")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
            log_waits: status_vars.get("Innodb_log_waits")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
            row_lock_waits: status_vars.get("Innodb_row_lock_waits")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
        };

        Ok(MySQLMetrics {
            connections,
            queries,
            innodb,
            replication: None, // Would need to check SHOW SLAVE STATUS
            slow_queries: status_vars.get("Slow_queries")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
            uptime: status_vars.get("Uptime")
                .and_then(|v| v.parse().ok()).unwrap_or(0),
            version: status_vars.get("version")
                .unwrap_or(&"Unknown".to_string()).clone(),
        })
    }

    async fn collect_postgresql_metrics(&self) -> Result<PostgreSQLMetrics> {
        let _connection_string = format!(
            "host={} port={} user={} password={} dbname={}",
            self.config.postgresql_host,
            self.config.postgresql_port,
            self.config.postgresql_user,
            self.config.postgresql_password,
            self.config.postgresql_database
        );

        // This would require postgres client library integration
        // For now, return mock data
        Ok(PostgreSQLMetrics {
            connections: ConnectionMetrics {
                current: 10,
                max: 100,
                total_created: 1000,
                aborted: 5,
            },
            database_stats: vec![
                DatabaseStats {
                    name: "postgres".to_string(),
                    size: 1024 * 1024 * 100, // 100MB
                    connections: 5,
                    transactions_per_second: 10.5,
                }
            ],
            locks: LockMetrics {
                waiting: 2,
                granted: 50,
                deadlocks: 1,
            },
            replication: None,
            cache_hit_ratio: 0.95,
            version: "14.0".to_string(),
        })
    }

    async fn collect_mongodb_metrics(&self) -> Result<MongoDBMetrics> {
        // This would require MongoDB client library integration
        // For now, return mock data
        Ok(MongoDBMetrics {
            connections: ConnectionMetrics {
                current: 25,
                max: 200,
                total_created: 5000,
                aborted: 10,
            },
            operations: OperationMetrics {
                queries: 10000,
                inserts: 5000,
                updates: 3000,
                deletes: 1000,
                getmores: 2000,
                commands: 15000,
            },
            memory: MongoMemoryMetrics {
                resident: 1024 * 1024 * 512, // 512MB
                virtual_mem: 1024 * 1024 * 1024, // 1GB
                mapped: 1024 * 1024 * 256, // 256MB
                cache_size: 1024 * 1024 * 128, // 128MB
            },
            replication: None,
            sharding: None,
            version: "5.0".to_string(),
        })
    }

    async fn collect_redis_metrics(&self) -> Result<RedisMetrics> {
        // This would require Redis client library integration
        // For now, return mock data
        Ok(RedisMetrics {
            connections: ConnectionMetrics {
                current: 15,
                max: 10000,
                total_created: 2500,
                aborted: 3,
            },
            memory: RedisMemoryMetrics {
                used: 1024 * 1024 * 64, // 64MB
                peak: 1024 * 1024 * 80, // 80MB
                rss: 1024 * 1024 * 70, // 70MB
                fragmentation_ratio: 1.2,
            },
            keyspace: KeyspaceMetrics {
                total_keys: 50000,
                expires: 10000,
                expired_keys: 500,
                evicted_keys: 100,
            },
            persistence: PersistenceMetrics {
                rdb_last_save_time: 1640995200, // Unix timestamp
                rdb_changes_since_last_save: 1000,
                aof_enabled: true,
                aof_size: 1024 * 1024 * 10, // 10MB
            },
            replication: Some(RedisReplicationMetrics {
                role: "master".to_string(),
                connected_slaves: 2,
                master_repl_offset: 123456789,
            }),
            version: "6.2".to_string(),
        })
    }

    pub fn get_database_summary(&self) -> Vec<String> {
        let mut summary = Vec::new();
        
        if let Some(metrics) = &self.last_metrics {
            if let Some(mysql) = &metrics.mysql {
                summary.push(format!("MySQL: {} connections, {:.1} QPS", 
                    mysql.connections.current, mysql.queries.queries_per_second));
            }
            
            if let Some(pg) = &metrics.postgresql {
                summary.push(format!("PostgreSQL: {} connections, {:.1}% cache hit", 
                    pg.connections.current, pg.cache_hit_ratio * 100.0));
            }
            
            if let Some(mongo) = &metrics.mongodb {
                summary.push(format!("MongoDB: {} connections, {}MB memory", 
                    mongo.connections.current, mongo.memory.resident / (1024 * 1024)));
            }
            
            if let Some(redis) = &metrics.redis {
                summary.push(format!("Redis: {} keys, {}MB memory", 
                    redis.keyspace.total_keys, redis.memory.used / (1024 * 1024)));
            }
        }
        
        summary
    }
}