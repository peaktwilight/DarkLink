import { ref, reactive } from 'vue'

// Mock data for development when backend is not available
export function useMockData() {
  const mockAgents = ref([
    {
      id: 'agent-001',
      type: 'Standard',
      connected: true,
      last_seen: new Date().toISOString(),
      ip: '192.168.1.100',
      hostname: 'DESKTOP-ABC123',
      os: 'Windows 10 Pro'
    },
    {
      id: 'agent-002', 
      type: 'Debug',
      connected: false,
      last_seen: new Date(Date.now() - 300000).toISOString(),
      ip: '10.0.0.50',
      hostname: 'ubuntu-server',
      os: 'Ubuntu 20.04 LTS'
    }
  ])

  const mockListeners = ref([
    {
      id: 'listener-001',
      config: {
        id: 'listener-001',
        name: 'base-http',
        protocol: 'http',
        host: '0.0.0.0',
        port: 8443,
        bindHost: '0.0.0.0'
      },
      status: 'Active',
      startTime: new Date(Date.now() - 3600000).toISOString()
    },
    {
      id: 'listener-002',
      config: {
        id: 'listener-002', 
        name: 'secure-https',
        protocol: 'https',
        host: '0.0.0.0',
        port: 8444,
        bindHost: '0.0.0.0'
      },
      status: 'Stopped'
    }
  ])

  const mockEvents = ref([
    {
      id: 1,
      timestamp: new Date().toISOString(),
      severity: 'INFO',
      message: 'Server started successfully',
      source: 'system'
    },
    {
      id: 2,
      timestamp: new Date(Date.now() - 60000).toISOString(),
      severity: 'SUCCESS',
      message: 'Agent agent-001 connected from 192.168.1.100',
      source: 'server'
    },
    {
      id: 3,
      timestamp: new Date(Date.now() - 120000).toISOString(),
      severity: 'WARNING',
      message: 'Agent agent-002 connection timeout',
      source: 'server'
    }
  ])

  const mockCommandResults = ref([
    {
      id: 1,
      timestamp: new Date(Date.now() - 300000).toISOString(),
      command: 'whoami',
      output: 'DESKTOP-ABC123\\user'
    },
    {
      id: 2,
      timestamp: new Date(Date.now() - 240000).toISOString(),
      command: 'pwd',
      output: 'C:\\Users\\user'
    }
  ])

  const mockFiles = ref([
    {
      name: 'payload.exe',
      size: 2048576,
      modified: new Date(Date.now() - 86400000).toISOString()
    },
    {
      name: 'config.json',
      size: 1024,
      modified: new Date(Date.now() - 3600000).toISOString()
    },
    {
      name: 'logs.txt',
      size: 4096,
      modified: new Date(Date.now() - 1800000).toISOString()
    }
  ])

  const mockBuildLogs = ref([
    {
      id: 1,
      timestamp: new Date().toISOString(),
      message: 'Starting payload generation...',
      type: 'info'
    },
    {
      id: 2,
      timestamp: new Date(Date.now() + 1000).toISOString(),
      message: 'Compiling Rust agent...',
      type: 'info'
    },
    {
      id: 3,
      timestamp: new Date(Date.now() + 5000).toISOString(),
      message: 'Payload generated successfully!',
      type: 'success'
    }
  ])

  return {
    mockAgents,
    mockListeners,
    mockEvents,
    mockCommandResults,
    mockFiles,
    mockBuildLogs
  }
}