# Daemon Design

## Program Layout

1. Configuration
    1. IO
    2. Management
2. Metrics
    1. Collection
    2. Storage
    3. Broadcasting
3. Console
    1. Management
    2. Communication
    3. Messaging
4. Dashboard
    1. Communication
    2. Messages
4. Logging

## Thread Layout
There are several thread groups to the regis platform:

1. Manager Thread
2. Console Manager: 
3. Console Worker(s)
4. Dashboard Manager: Sets up network interface for dashboard (client) connections
5. Dashbord Worker(s): A specific connection to a client
6. Metrics Collector: Determines information from the 
7. Metrics Broadcaster 