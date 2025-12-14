# Regis To-Do

## Overall Program Tasks
1. Implement Encryption - Done
2. Implement Authentication - In Progress
    1. Create authentication engine in regisd.
    2. Create commands to connect to regisc, so that it can see requests, and approve them.
        1. Write an integration test that performs such an action and ensures the correct result is made.
    3. Require authorization on the regis client.
        1. Enable communication with the OS-specific keyring.
        2. Enable communication handshake with regisd to run such a command.
3. Unify regis client in the common library, such that it has a dedicated backend like regisc.
4. Build Regisc CLI
5. Build Regis client CLI 
6. Build Regis client GUI

