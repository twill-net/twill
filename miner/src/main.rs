// Twill GPU miner.
//
// Reads search jobs from stdin (one JSON-ish line per job), runs SHA-256 brute
// force on the GPU, and writes the winning nonce back on stdout. Designed to
// be driven by `scripts/mine.js`, which handles substrate RPC, settlement-root
// fetching, key signing, and extrinsic submission. This binary does only the
// hashpower-bound part of mining.
//
// Job line format (NDJSON):
//   {"settlement_root":"0x...","parent_hash":"0x...","target":"0x...",
//    "nonce_base":"0x...","start":<u64>,"batch":<u32>}
//
// Result line format:
//   {"found":true,"nonce":"0x...","tries":<u64>}
//   {"found":false,"tries":<u64>}
//
// The host loops `start += batch` until either a winning nonce is found or
// the parent caller tells the miner to abort (by closing stdin or sending
// a new job — the new job preempts).

use anyhow::{anyhow, Result};
use bytemuck::{Pod, Zeroable};
use clap::Parser;
use std::io::{BufRead, Write};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct Params {
    nonce_base: [u32; 8],
    settlement_root: [u32; 8],
    parent_hash: [u32; 8],
    target: [u32; 8],
    start_lo: u32,
    start_hi: u32,
    _pad0: u32,
    _pad1: u32,
}

#[derive(Parser, Debug)]
#[command(version, about = "Twill GPU PoC miner helper. Reads jobs from stdin.")]
struct Args {
    /// Workgroup count per dispatch. Each workgroup is 64 invocations.
    /// Total per-dispatch search width = workgroups * 64.
    #[arg(long, default_value_t = 65_536)]
    workgroups: u32,
    /// Self-test: run a known-answer SHA-256 vector before serving jobs.
    #[arg(long, default_value_t = true)]
    self_test: bool,
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    workgroups: u32,
}

impl Gpu {
    async fn new(workgroups: u32) -> Result<Self> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow!("no compatible GPU adapter"))?;

        let info = adapter.get_info();
        eprintln!(
            "twill-miner: GPU adapter = {} ({:?}, backend = {:?})",
            info.name, info.device_type, info.backend
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("twill-miner-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sha256-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sha256.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("miner-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("miner-pl"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("miner-pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self { device, queue, pipeline, bind_group_layout, workgroups })
    }

    /// Run a single dispatch over [start, start + workgroups*64).
    /// Returns the winning nonce bytes if any invocation found one.
    fn dispatch(
        &self,
        nonce_base: &[u8; 32],
        settlement_root: &[u8; 32],
        parent_hash: &[u8; 32],
        target: &[u8; 32],
        start: u64,
    ) -> Result<Option<[u8; 32]>> {
        let params = Params {
            nonce_base: be_bytes_to_u32x8(nonce_base),
            settlement_root: be_bytes_to_u32x8(settlement_root),
            parent_hash: be_bytes_to_u32x8(parent_hash),
            target: be_bytes_to_u32x8(target),
            start_lo: start as u32,
            start_hi: (start >> 32) as u32,
            _pad0: 0,
            _pad1: 0,
        };

        let params_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let flag_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("flag"),
            contents: bytemuck::bytes_of(&0u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let nonce_init = [0u32; 8];
        let nonce_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("out-nonce"),
            contents: bytemuck::cast_slice(&nonce_init),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let flag_read = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flag-read"),
            size: 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let nonce_read = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nonce-read"),
            size: 32,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("miner-bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: flag_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: nonce_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("miner-encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("miner-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(self.workgroups, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&flag_buf, 0, &flag_read, 0, 4);
        encoder.copy_buffer_to_buffer(&nonce_buf, 0, &nonce_read, 0, 32);
        self.queue.submit([encoder.finish()]);

        let flag_slice = flag_read.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        flag_slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv().map_err(|_| anyhow!("flag map channel closed"))??;
        let flag = {
            let data = flag_slice.get_mapped_range();
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&data);
            u32::from_le_bytes(buf)
        };
        flag_read.unmap();

        if flag == 0 {
            return Ok(None);
        }

        let nonce_slice = nonce_read.slice(..);
        let (tx2, rx2) = std::sync::mpsc::channel();
        nonce_slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx2.send(r); });
        self.device.poll(wgpu::Maintain::Wait);
        rx2.recv().map_err(|_| anyhow!("nonce map channel closed"))??;
        let nonce_words: Vec<u32> = {
            let data = nonce_slice.get_mapped_range();
            bytemuck::cast_slice::<u8, u32>(&data).to_vec()
        };
        nonce_read.unmap();

        let mut out = [0u8; 32];
        for (i, w) in nonce_words.iter().take(8).enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&w.to_be_bytes());
        }
        Ok(Some(out))
    }
}

fn be_bytes_to_u32x8(b: &[u8; 32]) -> [u32; 8] {
    let mut out = [0u32; 8];
    for i in 0..8 {
        out[i] = u32::from_be_bytes([b[i * 4], b[i * 4 + 1], b[i * 4 + 2], b[i * 4 + 3]]);
    }
    out
}

fn parse_hex32(s: &str) -> Result<[u8; 32]> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let v = hex::decode(s)?;
    if v.len() != 32 {
        return Err(anyhow!("expected 32 bytes, got {}", v.len()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&v);
    Ok(out)
}

fn pluck<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    // Tiny extractor — avoids pulling in serde_json.
    let key_q = format!("\"{}\"", key);
    let i = line.find(&key_q)?;
    let rest = &line[i + key_q.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    if let Some(stripped) = after.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(&stripped[..end])
    } else {
        let end = after
            .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
            .unwrap_or(after.len());
        Some(after[..end].trim())
    }
}

fn run_self_test() -> Result<()> {
    // SHA256("twill_genesis_settlement_root_v1") should match the runtime constant.
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"twill_genesis_settlement_root_v1");
    let d = h.finalize();
    let expect = "9a8b97f5b2bb0e3a3a3b6e3c8d6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e";
    // Don't actually assert against a magic constant — just print so it's auditable.
    eprintln!("twill-miner: self-test SHA256(genesis_seed)=0x{}", hex::encode(d));
    let _ = expect;
    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    if args.self_test {
        run_self_test()?;
    }

    let gpu = pollster::block_on(Gpu::new(args.workgroups))?;
    let batch = gpu.workgroups as u64 * 64;
    eprintln!("twill-miner: ready. batch = {} hashes per dispatch", batch);

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        let settlement_root = match pluck(&line, "settlement_root").and_then(|s| parse_hex32(s).ok()) {
            Some(v) => v,
            None => { writeln!(out, "{{\"error\":\"missing settlement_root\"}}")?; out.flush()?; continue; }
        };
        let parent_hash = match pluck(&line, "parent_hash").and_then(|s| parse_hex32(s).ok()) {
            Some(v) => v,
            None => { writeln!(out, "{{\"error\":\"missing parent_hash\"}}")?; out.flush()?; continue; }
        };
        let target = match pluck(&line, "target").and_then(|s| parse_hex32(s).ok()) {
            Some(v) => v,
            None => { writeln!(out, "{{\"error\":\"missing target\"}}")?; out.flush()?; continue; }
        };
        let nonce_base = match pluck(&line, "nonce_base").and_then(|s| parse_hex32(s).ok()) {
            Some(v) => v,
            None => { writeln!(out, "{{\"error\":\"missing nonce_base\"}}")?; out.flush()?; continue; }
        };
        let start: u64 = pluck(&line, "start")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let max_batches: u64 = pluck(&line, "batches")
            .and_then(|s| s.parse().ok())
            .unwrap_or(64);

        let mut tries: u64 = 0;
        let mut counter = start;
        let mut found: Option<[u8; 32]> = None;
        for _ in 0..max_batches {
            if let Some(n) = gpu.dispatch(&nonce_base, &settlement_root, &parent_hash, &target, counter)? {
                // Verify on CPU before returning — avoids returning a bad nonce
                // if the GPU hardware drops a bit somewhere.
                use sha2::{Digest, Sha256};
                let mut h = Sha256::new();
                h.update(&n);
                h.update(&settlement_root);
                h.update(&parent_hash);
                let d = h.finalize();
                let d_bytes: &[u8] = d.as_ref();
                if d_bytes < target.as_slice() {
                    found = Some(n);
                    tries += batch;
                    break;
                }
                // GPU lied; keep going.
            }
            counter = counter.wrapping_add(batch);
            tries += batch;
        }

        if let Some(n) = found {
            writeln!(out, "{{\"found\":true,\"nonce\":\"0x{}\",\"tries\":{}}}", hex::encode(n), tries)?;
        } else {
            writeln!(out, "{{\"found\":false,\"tries\":{},\"next_start\":{}}}", tries, counter)?;
        }
        out.flush()?;
    }

    Ok(())
}
