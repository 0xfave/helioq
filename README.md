# Helioq

Helioq is a Solana program that enables server registration, metrics tracking, and reward distribution for server operators. It provides a decentralized way to manage and incentivize server performance.

## Features

- **Server Registration**: Register servers with unique IDs and assign owners
- **Metrics Submission**: Track server performance through uptime and task completion metrics
- **Reward System**: Distribute rewards based on server performance
- **Grace Period**: New servers get a 7-day grace period after registration
- **Server Management**: Support for server deactivation and ownership reassignment
- **Admin Controls**: Program can be paused by admin if needed

## Smart Contract Structure

### Core Accounts

1. **AdminAccount**
   - Manages program authority
   - Tracks the reward pool
   - Controls program pause state

2. **Server**
   - Stores server information and metrics
   - Tracks pending rewards
   - Manages server state (active/inactive)

### Key Instructions

1. **initialize**
   - Initializes the admin account
   - Sets up program authority

2. **registerServer**
   - Registers a new server with a unique ID
   - Sets initial grace period of 7 days
   - Maximum server ID length: 32 characters

3. **submitMetrics**
   - Updates server performance metrics
   - Accumulates reward points
   - Requires uptime percentage (0-100)

4. **claimRewards**
   - Claims accumulated rewards after cooldown period
   - 7-day cooldown between claims
   - Transfers SOL from program to server owner

5. **depositRewards**
   - Adds SOL to the reward pool
   - Only callable by authority

6. **reclaimStaleRewards**
   - Reclaims rewards from inactive servers
   - 365-day inactivity threshold

7. **deactivateServer**
   - Deactivates a server
   - Prevents further metrics submission

8. **reassignServer**
   - Transfers server ownership
   - Only works for active servers

## Development

### Prerequisites

- Rust 1.68.0 or later
- Solana Tool Suite
- Anchor Framework
- Node.js and npm/yarn

### Building
