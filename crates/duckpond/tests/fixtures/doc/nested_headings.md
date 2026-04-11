# Data flow architecture

How data moves between the web client, API gateway, workers, and
persistent stores.

## Overview

The system uses request-response for sync operations and message
queues for async work.

## Synchronous path

### Step 1

Client request arrives at the API gateway.

### Step 2

Gateway validates auth and rate limits.

## Asynchronous path

### Step 1

Client request arrives at the API gateway.

### Step 2

Service enqueues a job on the message queue.

### Step 3

Worker picks up the job and processes it.
