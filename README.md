golem-des
---

Golem-des is a Golem marketplace agent-based DES simulator. It is currently designed to simulate the market with usages that will be standard in Golem with Golem Clay release.

## Using the simulator

### Building
First of all, you'll need to install the required prerequisities which are `libgsl` and `gnuplot`. On Ubuntu, this can be accomplished by running the following command

```
$ sudo apt install gnuplot libgsl23 libgsl-dev
```

To then build the simulator in release mode, run in the command line

```
$ cargo build --release
```

To test it, run

```
$ cargo test
```

### Running
After the simulator has been built, it can be executed from the command line as follows (here, we assume you're at the top of the crate, i.e., in `golem-des`)

```
$ ./target/release/gm_des <some-simulation-scenario-in-json> --repetitions=100 --output-dir=<output-dir>
```

By default, the simulator will repeat the scenario for 100 times, hence, `--repetitions` can be omitted unless you want to run the simulation for a specific number of times.

By default, the simulator will save the resultant statistics in the current working directory. If you want to specify an alternative directory, pass it as an optional argument '--output-dir'.

### Specifying the simulation scenario
The only required argument for the simulator is the simulation scenario in JSON format as evidenced in the example invocation above. Several example scenarios in JSON format can be found in [scenarios/](scenarios) directory. However, the general structure can be summarised as follows

```txt
{
  "seed": 42,                         // starting seed to the PRNG; each subsequent repetition
                                      // gets seed++
                                            
  "duration": 604800,                 // simulated duration in seconds
    
  "providers": [                      // list of individual providers with parameters
                                      // specified manually; each such provider will exist in
                                      // __all__ simulation repetitions
    {
      "min_price": 0.000005,          // minimum price of the provider in GNT per CPU second
                                            
      "usage_factor": 0.01,           // usage factor of the provider; this value is
                                      // dimensionless as it represents a ratio of the
                                      // provider's CPU "speed" to the CPU of the
                                      // reference requestor characterised by usage of 1.0

      "behaviour": "regular"          // provider's behaviour; if the value is missing, by
                                      // default, the behaviour is then assummed to be
                                      // "regular";
                                      // possible values are:
                                      //  regular                - regular, truthful provider
                                      //  linear_usage_inflation - provider who linearly
                                      //                           inflates reported usage
                                      //  undercut_budget        - provider who always
                                      //                           reports budget minus some
                                      //                           epsilon
    }
  ],
  "provider_sources": [               // a list of randomised sources of the providers
    {                                 // each source randomly spawns new providers in
                                      // __each__ simulation repetition
    
      "provider_count": 49,           // number of providers to spawn in each simulation
                                      // repetition
                                            
      "min_price": {                  // minimum price distribution specification;
        "fixed": 0.00001              // possible values are:
      },                              //  fixed: value            - constant value generator
                                      //                            with required param
                                      //                            __value__
                                      //  choice: [values...]     - random choice from a
                                      //                            sequence __values__
                                      //  uniform: [min, max]     - uniform distribution
                                      //                            with __min__ and __max__
                                      //  lognormal: [mean, std]  - lognormal distribution
                                      //                            with __mean__ and __std__
                                      //  normal: [mean, std]     - normal distribution with
                                      //                            __mean__ and __std__
                                      //  exp: mean               - negative exponential
                                      //                            distribution with __mean__
                                            
      "usage_factor": {               // usage factor distribution specification
        "lognormal": [0.0, 1.0]       // specified similarly to minimum price (cf. above)
      }
    }
  ]
  "requestors": [                     // list of individual requestors with parameters
                                      // specified manually; each such requestor will
                                      // exist in __all__ simulation repetitions
    {
      "max_price": 0.001,             // maximum price of the requestor in GNT per
                                      // CPU second
                                            
      "budget_factor": 0.5,           // budget factor of the requestor; used to calculate
                                      // the requestor's budget per subtask according to
                                      // the formula:
                                      //  budget_factor * max_price * subtask_nominal_usage
                                            
      "tasks": [                      // list of tasks with parameters specified manually
        {
          "subtask_count": 200,       // count of subtasks in this task
                    
          "nominal_usage": {          // nominal usage of each subtask in CPU seconds
            "normal": [200, 10]
          }
        },
        {
          "subtask_count": 10,
          "nominal_usage": {
            "fixed": 1000
          }
        }
      ],
      "repeating": true               // whether the tasks should be respawned indefinitely
                                      // after the previous task is completed; by default,
                                      // when specyfing the requestor manually, this flag
                                      // is set to __false__
    }
  ],
  "requestor_sources": [              // list of randomised sources of the requestors
    {                                 // each source randomly spawns new requestors in
                                      // __each__ simulation repetition
                                            
      "requestor_count": 49,          // number of requestors to spawn in __each__
                                      // simulation repetition
                                            
      "max_price": {                  // maximum price distribution specification
        "fixed": 0.0001
      },
      "budget_factor": {              // budget factor distribution specification
        "choice": [0.37, 1.0, 2.72]
      },
      "subtask_count": {              // count of subtasks per task specification
        "uniform": [10, 100]
      },
      "nominal_usage": {              // subtask's nominal usage specification;
        "uniform": [100, 3600]        // NB the nominal usage values are drawn
      }                               // from the specified distribution for each
                                      // generated requestor
    }
  ]
}
```

### Analysing the output
As already mentioned, by default, the simulator will output CSV files with gathered statistics in the directory where the simulator was run from. Currently, there are 2 CSV files generated, one with statistics related to providers, and one with statistics related to requestors.

In case of providers, the CSV files contains the following columns

```txt
min_price,usage_factor,profit_margin,price,revenue,num_subtasks_assigned,num_subtasks_computed,num_subtasks_cancelled
0.00001,0.9880009379706052,9.265960685380145,0.00010265960685380146,26.790677283395098,415,232,182
0.00001,0.9742003077882163,10.246742693436717,0.00011246742693436718,28.25978137866988,473,248,224
0.00001,0.3208405263278145,30.812674573427067,0.0003181267457342707,58.90975497394528,1257,892,364
0.00001,0.9033102078285482,11.88548890207499,0.00012885488902074992,31.398507100223867,463,255,207
0.00001,0.21567254514292833,45.98127550501441,0.0004698127550501441,87.39514529618569,1808,1304,503
```

Whereas, in case of requestors, it is the following

```txt
max_price,budget_factor,num_tasks_advertised,num_tasks_computed,num_readvertisements,num_subtasks_computed,num_subtasks_cancelled
0.0001,0.37,6,5,36331,285,475
0.0001,0.37,5,4,35867,314,501
0.0001,2.72,18,17,36118,751,0
0.0001,2.72,14,13,36594,795,0
0.0001,1,54,53,32949,538,127
```

This way, as the user of the simulator, you are not constrained to Rust for further (statistical) processing of the simulation output. However, for your convenience, a basic analysis tool is bundled with the simulator. It can be invoked from the command line by running

```
./target/release/analyse (providers|requestors) <csv-file>
```

NB you need gnuplot binary installed and in your PATH in order to run the `analyse` binary.

The analysis tool currently aggregates the providers and requestors by their usage factor and budget factor values respectively. It then calculates and plots the following summary statistics (all with 99% confidence intervals depicted in the figures as error bars):

* For providers: mean (end) price, mean (end) effective price (that is, price times usage factor), and mean revenue.
* For requestors: mean ratio of subtasks cancelled to subtasks computed.

## Development
To build the simulator in debug mode, run in the command line

```
$ cargo build
```

To test it, run

```
$ cargo test
```

## License
[GPL-3.0](../../../LICENSE.txt)
