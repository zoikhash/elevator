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

Run `bash scripts/generateInputData.sh --help` for parameter limits and usage.


---

### Notes

Basement? (shift floors)

express elevators - sparse matrix definition

id cards and restricted floor access

Some passengers more likely than others?

door opening/closing times

elevator speedup/slowdown times - increase amount of quantisation. differing floor heights

If lift is moving fast, cannot stop within X number of floors

elevator capacities

elevator energy usage
