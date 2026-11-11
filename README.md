# Elevator simulation

This codebase simulates the operation of an elevator.  

## Quickstart

First-time setup:

```bash
# ensure "python" is available on your OS's path

# create a virtual environment in elevator/.env
python3 -m venv .venv
```

Quickstart:

```bash
# activate the virtual Python environment
source .venv/bin/activate

# generate simulated input data
./scripts/generateInputData.sh

# install Python dependencies
python -m pip install -r requirements.txt

# open the notebook
python -m jupyterlab elevator.ipynb
# alternatively, use VSCode extension for Jupyter notebook development

# run the simulation from Jupyter

# outputs (elevator_position_history.csv and summary_stats.txt) can be viewed in:
ls -altr ./out/
```

## Generate simulation input

As per description in ./resources/Take_home_Elevator.pdf, generate some simulation input.

The Bash generator requires `shuf` (GNU coreutils), but no Python environment:

```bash
bash scripts/generateInputData.sh \
  --nPassengerIds 100 \
  --nFloors 50 \
  --simulationLength 1000 > input.csv
```

All three parameters are required. The CSV contains `time,id,source,dest`, with
requests generated in chronological order from a pool of `nPassengerIds` IDs.
IDs can repeat, but requests from the same passenger are at least 10 epochs apart.
Not every passenger is guaranteed to request a trip. Times range from 0 to
`simulationLength - 1`; floors range from 0 to `nFloors - 1`. Source floors are
uniformly random, and destinations are uniformly random among the other floors.

Normally, each epoch has one request opportunity with probability 0.25.
Override this with `--requestProbability 0.5` (range 0 to 1, up to four decimal
places). Add repeatable `--busyPeriod 100:200:20` options for 20 independent
opportunities per epoch from time 100 through 199. This can generate multiple
requests per epoch and increase the CSV length without adding passenger IDs.
Requests are skipped when every passenger is on cooldown. Cooldown measures
time since the last request, not journey completion; trip durations and passenger
location continuity are not modeled by this generator.

Add `--groundFloorProbability 70` to send passengers starting above floor 0 to
floor 0 with 70% probability. Otherwise, they choose a different nonzero floor.
Passengers starting on floor 0 choose a nonzero destination. With only two floors,
the destination is always the other floor, regardless of the probability.

Run `bash scripts/generateInputData.sh --help` for parameter limits and usage.


---

### Notes

Basement? (shift floors)

express elevators - sparse matrix definition

id cards and restricted floor access

Some passengers more likely than others?

door opening/closing times

elevator speedup/slowdown times - increase amount of quantisation. differing floor heights