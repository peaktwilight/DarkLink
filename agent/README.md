# MicroC2 Agent

**A memory-resident, OPSEC-focused implant** for secure command execution and file transfer, designed to operate with minimal forensic footprint while evading modern detection mechanisms.

```rust
[ Zero DLL Dependencies | NTAPI Syscalls Only | Memory-Only Execution ]
```

## Agent Abilities
    1. transfer files
    2. Perform command exectution
        - Use NTAPI syscalls directly (no CreateProcess)
        - Execute through trusted processes (rundll32.exe, msiexec.exe)

## Stealth (currently not Implemented)
### OPSEC modes
#### Full OPSEC (User actively using the machine)
    
    Agent minimizes its behavioral footprint and avoids any perceptible impact

#### Background OPSEC (User Idle/AFK, screen locked, no input for a while, off hours)
    
    Agent can become more active, still remain stealthy
        - Run Queued commands
        - perform file transfer
        - more "risky" command execution

#### Adaptive behaviour
        
    - Detect user idleness via system APIs (e.g. GetLastInputInfoto check inactivity timing, listen for workstation lock events) switch to mode
    - Mode switching the moment activity is detected

### Evasiontechniques (for later)

##### IMPORTANT DESIGN THE BEHAVIOUR THAT MAKES SENSE IN CONTEXT WITH NORMAL SYSTEM OPERATIONS


#### Direct Syscalls (Bypass User-Mode Hooks)

#### Indirect Syscalls (Return Address Spoofing)

#### API Unhooking

#### Dynamic API Resolution

#### Memory Protection Tricks
Avoid RWX memory, which is heavily monitored:

#### Obfuscation & Encryption

#### Legitimate Process Ause (Living-Off-The-Land)

#### Reflective DLL Injection

#### Spoofed Stack Traces

#### Minimize Suspicious Patterns

    Delay Execution: Add sleep calls between sensitive operations.
    Split Payloads: Use stagers to fetch payloads in chunks.

Avoid Known Bad APIs:

    Replace VirtualAlloc → NtAllocateVirtualMemory
    Replace CreateRemoteThread → RtlCreateUserThread



