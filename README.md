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

```

## Generate simulation input

As per description in ./resources/Take_home_Elevator.pdf, generate some simulation input.

The Bash generator requires `shuf` (GNU coreutils) and `sort`, but no Python environment:

```bash
bash scripts/generateInputData.sh \
  --nPassengerIds 100 \
  --nFloors 50 \
  --simulationLength 1000 > input.csv
```

All three parameters are required. The CSV contains `time,id,source,dest`, with
one request per unique passenger, sorted by time. Times range from 0 to
`simulationLength - 1`; floors range from 0 to `nFloors - 1`. Source floors are
uniformly random, and destinations are uniformly random among the other floors.

Add `--groundFloorProbability 70` to send passengers starting above floor 0 to
floor 0 with 70% probability. Otherwise, they choose a different nonzero floor.
Passengers starting on floor 0 choose a nonzero destination. With only two floors,
the destination is always the other floor, regardless of the probability.

Run `bash scripts/generateInputData.sh --help` for parameter limits and usage.


---

### Notes

Basement?

id cards and restricted floor access

Some passengers more likely than others?

