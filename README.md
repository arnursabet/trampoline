# Trampoline Framework
The framework for developing decentralized applications on Nervos Network's Common Knowledge Base.


# Motivation

The UTXO model is the primary alternative to the Account model. Nervos Network's state & programming model, termed the "cell"
model, is a generalization of the UTXO model to enhance programmability. Nervos Network is a UTXO-based smart contract platform.

There are myriad tradeoffs between UTXO and Account model - the choice depends on the use case and priorities of the developer.

One powerful feature of the UTXO model is that significant composition of smart contract behavior can be realized in a single transaction, where the end result of the transaction is known a priori (before the transaction is submitted). This is because a transaction is a *complete* description of the proposed state change. This is in contrast to an Account model wherein the transaction, post-submission, can have unexpected results depending on what state changes occur prior to its execution while it's still sitting in the mempool.

Yet, this comes at a cost: *transaction generation* in a UTXO model is a lot more cumbersome. It takes a lot of custom code because all of the custom smart contract logic essentially needs to be reimplemented off-chain a second time.

After implementing all of that logic, a developer would still have to implement even more code in order to create composable transactions, or transactions which include not just their own smart contract actions, but also the actions of 3rd party smart contracts (i.e., building the software that allows your contracts to interact with another system's contracts is just as cumbersome an undertaking as is building all of your dapp's logic in the first place).

The difficulty of building transaction generation logic is also consequential when testing: it is difficult to test the multitude of ways in which your smart contracts may interact (or not) with others, since building that interaction logic takes so much boilerplate.

Reducing the labor required to take advantage of composability on Nervos Network (the largest UTXO-based smart contract platform currently) is a primary goal of Trampoline.

To achieve this, Trampoline is in large part a transaction composition framework. The philosophy of Trampoline is that a smart contract is actually the *combination* of the on-chain scripts which *verify* conditions *and* the off-chain code which *generates* transactions to meet those conditions. Trampoline transaction generators take as inputs a pipeline of smart contracts and passes an empty transaction through this pipeline. Each smart contract then applies its respective updates to the entire transaction. This dramatically reduces the amount of code required when implementing a specific contract's logic. It also removes the burden of keeping track of all the various contract's logic when building composable transactions that include multiple smart contracts & their interactions.

Trampoline has other important goals as well, so here's a list of them:
1. Make composability far easier and quicker to achieve
2. Make transaction generation more declarative with less boilerplate
3. Enable easier end to end smart contract testing and simulation
4. Provide a single tool to streamline the entire dapp development experience
5. Support multi-network dapps and configurable network architectures


Goals 1 and 2 are achieved with Trampoline's pipeline-based approach to transaction generation. Goal 3 is also achieved by Trampoline's approach to composable transactions *as well as* Trampoline's simulation chain, which makes it possible to simulate end to end interactions over different time scales without even starting a network. Goal 4 is achieved with Trampoline's `create-react-app`-like experience to creating new dapp projects. Goal 5 is achieved by Trampoline's approach to orchestrating network services, which allows developers to spin up multiple nodes, miners, bridges to other chains, as well as layer 2 networks. This entire multi-chain, multi-layer environment can be configured by developers & launched locally in the time it takes to install Trampoline itself! Trampoline has simple defaults, but can easily scale the sophistication of the testing environment to fit the needs of the dapp developer, whether they're building a complex dapp on layer 1, a cross-chain dapp between Nervos Network and some other blockchain, or whether they're building on both layer 1 & layer 2.

# Features
Trampoline allows you to jump straight into dapp development without worrying about pesky configuration, installing
a bunch of different tools, etc.

Trampoline projects include all components of a dapp so that you can manage all of the moving parts in one place in
coherent development environment.

- [x] Quickly generate new trampoline projects.
- [x] Start and stop local dev chain with ease.
- [x] Start and stop local indexer(s) with ease.
- [x] Add your own miner(s).
- [x] Autogenerate Rust bindings for custom schemas for use on and off chain.
- [x] Contract API for partial transaction creation based on custom logic
- [x] Generator API for transaction generation with Contract pipelines
- [x] Simulation chain for testing without running external services (WIP)
- [ ]  Manage accounts and addresses across developer, staging, and deployment environments.
- [ ]  Indexer extensions to index custom schemas.
- [ ]  Trampoline server API powered by Rocket-rs for transaction generation & querying.
- [ ]  Declaratively define transaction patterns for easy transaction creation.
- [ ]  Compile transaction patterns to CKB scripts 


# Installation
Currently, just ensure the Rust toolchain is installed on your machine. Then, install with Cargo:

`cargo install trampoline --git https://github.com/WilfredTA/trampoline`

You also need to have Docker installed.

# Usage

## Start a new project
`trampoline new <project_name>`

## Manage local network

To initialize and start a new network: `trampoline network launch`

To set a miner: `trampoline network set-miner [lock_arg | pubkey]`

To start the miner: `trampoline network miner`

To start an indexer: `trampoline network indexer`

## Manage schemas

Create a new schema: `trampoline schema new <schema_name>`

Optionally, schema definition (in Molecule) can be passed in: `trampoline schema new byte_10_arr "array my_array [byte; 10]"`

Generate rust bindings to build and decode schema: `trampoline schema build <schema_name>`

