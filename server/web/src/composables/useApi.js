import { ref } from 'vue'
import { useMockData } from './useMockData.js'

const isDevelopment = import.meta.env.DEV
// Use mock data only when running on Vite dev server (port 3000)
// When on real server ports (8080/8443), always use real API
const USE_MOCK_DATA = isDevelopment && window.location.port === '3000'

console.log('🐛 API Mode:', {
  isDevelopment,
  port: window.location.port,
  USE_MOCK_DATA,
  location: window.location.href
})

export function useApi() {
  const loading = ref(false)
  const error = ref(null)
  const { mockAgents, mockListeners, mockFiles } = useMockData()

  async function makeRequest(url, options = {}) {
    loading.value = true
    error.value = null
    
    // Use mock data in development when backend is not available
    if (USE_MOCK_DATA) {
      return handleMockRequest(url, options)
    }
    
    // Debug: Log all API requests
    console.log('🌐 API Request:', {
      url,
      method: options.method,
      body: options.body ? JSON.parse(options.body) : null,
      headers: options.headers
    })
    
    try {
      const response = await fetch(url, {
        headers: {
          'Content-Type': 'application/json',
          ...options.headers
        },
        ...options
      })

      if (!response.ok) {
        const errorText = await response.text()
        let errorMessage
        try {
          const errorJson = JSON.parse(errorText)
          errorMessage = errorJson.error || errorJson.message || `HTTP ${response.status}`
        } catch {
          errorMessage = errorText || `HTTP ${response.status}`
        }
        throw new Error(errorMessage)
      }

      const contentType = response.headers.get('content-type')
      if (contentType && contentType.includes('application/json')) {
        return await response.json()
      } else {
        return await response.text()
      }
    } catch (err) {
      error.value = err.message
      // Fall back to mock data on network errors in development
      if (isDevelopment) {
        console.warn('API request failed, using mock data:', err.message)
        return handleMockRequest(url, options)
      }
      throw err
    } finally {
      loading.value = false
    }
  }

  function handleMockRequest(url, options = {}) {
    return new Promise((resolve) => {
      // Simulate network delay
      setTimeout(() => {
        if (url.includes('/api/agents/list')) {
          resolve(mockAgents.value.reduce((acc, agent) => {
            acc[agent.id] = agent
            return acc
          }, {}))
        } else if (url.includes('/api/listeners/list')) {
          resolve(mockListeners.value)
        } else if (url.includes('/api/file_drop/list')) {
          resolve(mockFiles.value)
        } else if (url.includes('/api/listeners/create') && options.method === 'POST') {
          const newListener = {
            id: 'listener-' + Date.now(),
            config: JSON.parse(options.body || '{}'),
            status: 'Active',
            startTime: new Date().toISOString()
          }
          mockListeners.value.push(newListener)
          resolve({ success: true })
        } else if (url.includes('/api/payload/generate')) {
          resolve({
            success: true,
            filename: 'agent.exe',
            size: 2048576,
            downloadUrl: '/mock/download/agent.exe'
          })
        } else {
          resolve({ success: true })
        }
        loading.value = false
      }, 300)
    })
  }

  async function apiGet(url, options = {}) {
    return makeRequest(url, {
      method: 'GET',
      ...options
    })
  }

  async function apiPost(url, data = null, options = {}) {
    return makeRequest(url, {
      method: 'POST',
      body: data ? JSON.stringify(data) : null,
      ...options
    })
  }

  async function apiPut(url, data = null, options = {}) {
    return makeRequest(url, {
      method: 'PUT',
      body: data ? JSON.stringify(data) : null,
      ...options
    })
  }

  async function apiDelete(url, options = {}) {
    return makeRequest(url, {
      method: 'DELETE',
      ...options
    })
  }

  async function apiPatch(url, data = null, options = {}) {
    return makeRequest(url, {
      method: 'PATCH',
      body: data ? JSON.stringify(data) : null,
      ...options
    })
  }

  return {
    loading,
    error,
    apiGet,
    apiPost,
    apiPut,
    apiDelete,
    apiPatch
  }
}