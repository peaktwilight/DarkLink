<template>
  <form @submit.prevent="generatePayload" class="payload-form">
    <!-- Basic Configuration -->
    <div class="form-section">
      <h3>Basic Configuration</h3>
      
      <div class="form-group">
        <label for="agentType">Agent Type</label>
        <select id="agentType" v-model="form.agentType" required class="form-select">
          <option value="agent">Agent (Release)</option>
          <option value="debugAgent">Agent (Debug)</option>
        </select>
      </div>

      <div class="form-group">
        <label for="listener">Listener</label>
        <select id="listener" v-model="form.listener" required class="form-select">
          <option value="">Select a listener...</option>
          <option 
            v-for="listener in listeners" 
            :key="getListenerId(listener)"
            :value="getListenerId(listener)"
          >
            {{ getListenerName(listener) }} ({{ getListenerProtocol(listener) }})
          </option>
        </select>
      </div>

      <div class="form-group">
        <label for="architecture">Architecture</label>
        <select id="architecture" v-model="form.architecture" required class="form-select">
          <option value="x64">x64</option>
          <option value="x86">x86</option>
          <option value="arm64">ARM64</option>
        </select>
      </div>

      <div class="form-group">
        <label for="format">Output Format</label>
        <select id="format" v-model="form.format" required class="form-select">
          <option value="windows_exe">Windows EXE</option>
          <option value="windows_dll">Windows DLL</option>
          <option value="windows_shellcode">Windows Shellcode</option>
          <option value="windows_service">Windows Service EXE</option>
          <option value="linux_elf">Linux ELF</option>
        </select>
      </div>
    </div>

    <!-- Agent Options -->
    <div class="form-section">
      <h3>Agent Options</h3>
      
      <div class="form-group">
        <label class="checkbox-label">
          <input v-model="form.opsec" type="checkbox">
          Enable OPSEC Mode
        </label>
      </div>

      <div class="form-group">
        <label for="sleep">Sleep Interval (seconds)</label>
        <input 
          id="sleep"
          v-model.number="form.sleep"
          type="number" 
          min="1"
          class="form-input"
        >
      </div>

      <div class="form-group">
        <label class="checkbox-label">
          <input v-model="form.indirectSyscall" type="checkbox">
          Enable Indirect Syscalls
        </label>
      </div>

      <div class="form-group">
        <label for="sleepTechnique">Sleep Technique</label>
        <select id="sleepTechnique" v-model="form.sleepTechnique" class="form-select">
          <option value="standard">Standard</option>
          <option value="winapi">WinAPI</option>
          <option value="modified">Modified/Obfuscated</option>
        </select>
      </div>
    </div>

    <!-- DLL Sideloading -->
    <div class="form-section">
      <h3>DLL Sideloading</h3>
      
      <div class="form-group">
        <label class="checkbox-label">
          <input v-model="form.dllSideloading" type="checkbox">
          Enable DLL Sideloading
        </label>
      </div>

      <div v-if="form.dllSideloading" class="sideloading-options">
        <div class="form-group">
          <label for="sideloadDll">Sideload DLL Name</label>
          <input 
            id="sideloadDll"
            v-model="form.sideloadDll"
            type="text" 
            class="form-input"
            placeholder="target.dll"
          >
        </div>

        <div class="form-group">
          <label for="exportName">Export Name</label>
          <input 
            id="exportName"
            v-model="form.exportName"
            type="text" 
            class="form-input"
            placeholder="DllMain"
          >
        </div>
      </div>
    </div>

    <!-- Communication Options -->
    <div class="form-section">
      <h3>Communication Options</h3>
      
      <div class="form-group">
        <label class="checkbox-label">
          <input v-model="form.socks5Enabled" type="checkbox">
          Enable SOCKS5 Configuration
        </label>
      </div>

      <div v-if="form.socks5Enabled" class="socks5-options">
        <div class="form-group">
          <label for="socks5Host">SOCKS5 Proxy Host</label>
          <input 
            id="socks5Host"
            v-model="form.socks5Host"
            type="text" 
            class="form-input"
          >
        </div>

        <div class="form-group">
          <label for="socks5Port">SOCKS5 Proxy Port</label>
          <input 
            id="socks5Port"
            v-model.number="form.socks5Port"
            type="number" 
            class="form-input"
          >
        </div>
      </div>
    </div>

    <!-- OPSEC Configuration -->
    <div v-if="form.opsec" class="form-section">
      <h3>OPSEC Configuration</h3>
      
      <div class="opsec-grid">
        <div class="form-group">
          <label for="procScanInterval">Process Scan Interval (s)</label>
          <input 
            id="procScanInterval"
            v-model.number="form.procScanIntervalSecs"
            type="number" 
            min="10"
            class="form-input"
          >
          <small class="form-help">Base interval for scanning processes</small>
        </div>

        <div class="form-group">
          <label for="thresholdEnterFull">Enter Full OPSEC Threshold</label>
          <input 
            id="thresholdEnterFull"
            v-model.number="form.baseThresholdEnterFullOpsec"
            type="number" 
            step="0.1"
            min="0"
            max="100"
            class="form-input"
          >
          <small class="form-help">Score (0-100) to enter Full OPSEC mode</small>
        </div>

        <div class="form-group">
          <label for="thresholdExitFull">Exit Full OPSEC Threshold</label>
          <input 
            id="thresholdExitFull"
            v-model.number="form.baseThresholdExitFullOpsec"
            type="number" 
            step="0.1"
            min="0"
            max="100"
            class="form-input"
          >
        </div>

        <div class="form-group">
          <label for="thresholdEnterReduced">Enter Reduced Activity Threshold</label>
          <input 
            id="thresholdEnterReduced"
            v-model.number="form.baseThresholdEnterReducedActivity"
            type="number" 
            step="0.1"
            min="0"
            max="100"
            class="form-input"
          >
        </div>

        <div class="form-group">
          <label for="thresholdExitReduced">Exit Reduced Activity Threshold</label>
          <input 
            id="thresholdExitReduced"
            v-model.number="form.baseThresholdExitReducedActivity"
            type="number" 
            step="0.1"
            min="0"
            max="100"
            class="form-input"
          >
        </div>

        <div class="form-group">
          <label for="minDurationFull">Min Duration Full OPSEC (s)</label>
          <input 
            id="minDurationFull"
            v-model.number="form.minDurationFullOpsecSecs"
            type="number" 
            min="0"
            class="form-input"
          >
        </div>
      </div>

      <!-- C2 Failure Thresholds -->
      <h4>Adaptive C2 Failure Thresholds</h4>
      <div class="opsec-grid">
        <div class="form-group">
          <label for="maxC2Failures">Max Consecutive C2 Failures</label>
          <input 
            id="maxC2Failures"
            v-model.number="form.baseMaxConsecutiveC2Failures"
            type="number" 
            min="1"
            class="form-input"
          >
        </div>

        <div class="form-group">
          <label for="c2ThresholdIncrease">C2 Threshold Increase Factor</label>
          <input 
            id="c2ThresholdIncrease"
            v-model.number="form.c2FailureThresholdIncreaseFactor"
            type="number" 
            step="0.01"
            min="1.0"
            class="form-input"
          >
        </div>

        <div class="form-group">
          <label for="c2ThresholdDecrease">C2 Threshold Decrease Factor</label>
          <input 
            id="c2ThresholdDecrease"
            v-model.number="form.c2FailureThresholdDecreaseFactor"
            type="number" 
            step="0.01"
            min="0.1"
            max="1.0"
            class="form-input"
          >
        </div>

        <div class="form-group">
          <label for="c2AdjustInterval">C2 Threshold Adjustment Interval (s)</label>
          <input 
            id="c2AdjustInterval"
            v-model.number="form.c2ThresholdAdjustIntervalSecs"
            type="number" 
            min="60"
            class="form-input"
          >
        </div>
      </div>
    </div>

    <!-- Form Actions -->
    <div class="form-actions">
      <Button 
        type="submit" 
        variant="primary"
        :loading="loading"
        :disabled="!isFormValid"
      >
        Generate Payload
      </Button>
      <Button 
        type="button" 
        variant="secondary"
        @click="resetForm"
      >
        Reset
      </Button>
    </div>
  </form>
</template>

<script setup>
import { ref, reactive, computed } from 'vue'
import Button from '../ui/Button.vue'

const props = defineProps({
  listeners: {
    type: Array,
    required: true
  },
  loading: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['generate'])

// Form data with defaults
const form = reactive({
  agentType: 'agent',
  listener: '',
  architecture: 'x64',
  format: 'windows_exe',
  opsec: false,
  sleep: 60,
  indirectSyscall: false,
  sleepTechnique: 'standard',
  dllSideloading: false,
  sideloadDll: '',
  exportName: '',
  socks5Enabled: false,
  socks5Host: '127.0.0.1',
  socks5Port: 9050,
  // OPSEC Configuration
  procScanIntervalSecs: 300,
  baseThresholdEnterFullOpsec: 60.0,
  baseThresholdExitFullOpsec: 60.0,
  baseThresholdEnterReducedActivity: 20.0,
  baseThresholdExitReducedActivity: 20.0,
  minDurationFullOpsecSecs: 300,
  minDurationReducedActivitySecs: 120,
  minDurationBackgroundOpsecSecs: 60,
  reducedActivitySleepSecs: 120,
  // C2 Failure Thresholds
  baseMaxConsecutiveC2Failures: 5,
  c2FailureThresholdIncreaseFactor: 1.1,
  c2FailureThresholdDecreaseFactor: 0.9,
  c2ThresholdAdjustIntervalSecs: 3600,
  c2DynamicThresholdMaxMultiplier: 2.0
})

const isFormValid = computed(() => {
  return form.listener && form.agentType && form.architecture && form.format
})

function getListenerId(listener) {
  return listener.config?.id || listener.id
}

function getListenerName(listener) {
  return listener.config?.name || listener.name || 'Unnamed'
}

function getListenerProtocol(listener) {
  return (listener.config?.protocol || listener.Protocol || 'Unknown').toUpperCase()
}

function generatePayload() {
  if (!isFormValid.value) return
  emit('generate', { ...form })
}

function resetForm() {
  Object.assign(form, {
    agentType: 'agent',
    listener: '',
    architecture: 'x64',
    format: 'windows_exe',
    opsec: false,
    sleep: 60,
    indirectSyscall: false,
    sleepTechnique: 'standard',
    dllSideloading: false,
    sideloadDll: '',
    exportName: '',
    socks5Enabled: false,
    socks5Host: '127.0.0.1',
    socks5Port: 9050,
    procScanIntervalSecs: 300,
    baseThresholdEnterFullOpsec: 60.0,
    baseThresholdExitFullOpsec: 60.0,
    baseThresholdEnterReducedActivity: 20.0,
    baseThresholdExitReducedActivity: 20.0,
    minDurationFullOpsecSecs: 300,
    minDurationReducedActivitySecs: 120,
    minDurationBackgroundOpsecSecs: 60,
    reducedActivitySleepSecs: 120,
    baseMaxConsecutiveC2Failures: 5,
    c2FailureThresholdIncreaseFactor: 1.1,
    c2FailureThresholdDecreaseFactor: 0.9,
    c2ThresholdAdjustIntervalSecs: 3600,
    c2DynamicThresholdMaxMultiplier: 2.0
  })
}
</script>

<style scoped>
.payload-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
}

.form-section {
  border: 1px solid var(--border-color);
  border-radius: var(--radius);
  padding: var(--space-4);
}

.form-section h3 {
  margin: 0 0 var(--space-4) 0;
  color: var(--accent-color);
  font-size: 1rem;
  font-weight: 600;
}

.form-section h4 {
  margin: var(--space-6) 0 var(--space-4) 0;
  color: var(--text-secondary);
  font-size: 0.875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.form-group {
  margin-bottom: var(--space-4);
}

.form-group:last-child {
  margin-bottom: 0;
}

.form-group label {
  display: block;
  margin-bottom: var(--space-2);
  font-weight: 500;
  color: var(--text-color);
}

.form-input,
.form-select {
  width: 100%;
  padding: var(--space-2) var(--space-3);
  background: var(--secondary-bg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius);
  color: var(--text-color);
  font-size: 0.875rem;
  transition: all 0.2s ease;
}

.form-input:focus,
.form-select:focus {
  outline: none;
  border-color: var(--accent-color);
  box-shadow: 0 0 0 2px rgba(88, 166, 255, 0.3);
}

.form-help {
  display: block;
  margin-top: var(--space-1);
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  cursor: pointer;
  font-weight: 500;
}

.checkbox-label input[type="checkbox"] {
  width: auto;
  margin: 0;
}

.sideloading-options,
.socks5-options {
  margin-top: var(--space-4);
  padding-top: var(--space-4);
  border-top: 1px solid var(--border-color);
}

.opsec-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: var(--space-4);
}

.form-actions {
  display: flex;
  gap: var(--space-3);
  justify-content: flex-end;
  padding-top: var(--space-4);
  border-top: 1px solid var(--border-color);
}

@media (max-width: 768px) {
  .opsec-grid {
    grid-template-columns: 1fr;
  }
}
</style>