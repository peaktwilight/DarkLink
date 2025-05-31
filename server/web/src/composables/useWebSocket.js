import { ref, onUnmounted } from 'vue'
import { useMockData } from './useMockData.js'

const isDevelopment = import.meta.env.DEV
const USE_MOCK_WS = isDevelopment && !window.location.port.includes('8080')

export function useWebSocket() {
  const isConnected = ref(false)
  const error = ref(null)
  const reconnectAttempts = ref(0)
  
  let ws = null
  let reconnectTimer = null
  let pingInterval = null
  let mockInterval = null
  
  const MAX_RECONNECT_ATTEMPTS = 10
  const RECONNECT_DELAY = 2000
  const PING_INTERVAL = 30000

  const { mockEvents } = useMockData()

  function connect(url, options = {}) {
    const {
      onMessage = () => {},
      onConnect = () => {},
      onDisconnect = () => {},
      onError = () => {}
    } = options

    // Use mock WebSocket in development when backend is not available
    if (USE_MOCK_WS) {
      return connectMock(url, options)
    }

    if (reconnectAttempts.value >= MAX_RECONNECT_ATTEMPTS) {
      error.value = 'Maximum reconnection attempts reached'
      onError(error.value)
      return
    }

    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const wsUrl = `${wsProtocol}//${window.location.host}${url}`
    
    if (ws) {
      ws.close()
    }

    ws = new WebSocket(wsUrl)
    
    const connectionTimeout = setTimeout(() => {
      if (ws.readyState !== WebSocket.OPEN) {
        ws.close()
        handleReconnect(url, options)
      }
    }, 5000)

    ws.onopen = () => {
      clearTimeout(connectionTimeout)
      isConnected.value = true
      error.value = null
      reconnectAttempts.value = 0
      onConnect()

      // Set up ping/pong to keep connection alive
      pingInterval = setInterval(() => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send('ping')
        } else {
          clearInterval(pingInterval)
        }
      }, PING_INTERVAL)
    }

    ws.onmessage = (event) => {
      if (event.data === 'pong') return
      onMessage(event.data)
    }

    ws.onclose = (event) => {
      clearTimeout(connectionTimeout)
      clearInterval(pingInterval)
      isConnected.value = false
      
      if (!event.wasClean) {
        handleReconnect(url, options)
      } else {
        onDisconnect()
      }
    }

    ws.onerror = () => {
      error.value = 'WebSocket connection error'
      onError(error.value)
    }
  }

  function handleReconnect(url, options) {
    const { onDisconnect = () => {} } = options
    
    reconnectAttempts.value++
    onDisconnect()

    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
    }

    reconnectTimer = setTimeout(() => {
      if (document.visibilityState === 'visible') {
        connect(url, options)
      }
    }, RECONNECT_DELAY)
  }

  function disconnect() {
    if (USE_MOCK_WS) {
      disconnectMock()
      return
    }
    
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
    }
    
    if (pingInterval) {
      clearInterval(pingInterval)
    }
    
    if (ws) {
      ws.close()
      ws = null
    }
    
    isConnected.value = false
  }

  function send(data) {
    if (USE_MOCK_WS) {
      console.log('🔧 Mock WebSocket send:', data)
      return true
    }
    
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(data)
      return true
    }
    return false
  }

  function connectMock(url, options = {}) {
    const {
      onMessage = () => {},
      onConnect = () => {},
      onDisconnect = () => {},
      onError = () => {}
    } = options

    console.log('🔧 Using mock WebSocket for development:', url)
    
    // Simulate connection
    setTimeout(() => {
      isConnected.value = true
      onConnect()
      
      // Send mock log messages for /ws/logs
      if (url.includes('/ws/logs')) {
        mockInterval = setInterval(() => {
          const mockLog = {
            timestamp: new Date().toISOString(),
            level: ['info', 'warning', 'error'][Math.floor(Math.random() * 3)],
            message: `Mock log message at ${new Date().toLocaleTimeString()}`
          }
          onMessage(JSON.stringify(mockLog))
        }, 5000)
      }
      
      // Send mock terminal responses for /ws/terminal
      if (url.includes('/ws/terminal')) {
        // Send initial connection message
        const initialResponse = {
          output: 'Connected to server terminal (Bash shell).\n',
          cwd: '~'
        }
        onMessage(JSON.stringify(initialResponse))
      }
    }, 500)
  }

  function disconnectMock() {
    if (mockInterval) {
      clearInterval(mockInterval)
      mockInterval = null
    }
    isConnected.value = false
  }

  onUnmounted(() => {
    disconnect()
    disconnectMock()
  })

  return {
    isConnected,
    error,
    reconnectAttempts,
    connect,
    disconnect,
    send
  }
}