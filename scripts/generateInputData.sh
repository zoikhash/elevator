#!/usr/bin/env bash
set -euo pipefail

# Generate one elevator request per passenger; write time-sorted CSV to stdout.

# Describe the required parameters and the optional destination weighting.
usage() {
    cat <<'HELP'
Usage:
  generateInputData.sh --nPassengerIds N --nFloors N --simulationLength N
                       [--groundFloorProbability PERCENT]

Required:
  --nPassengerIds N             Number of unique passengers (at least 1).
  --nFloors N                   Number of floors (at least 2), numbered 0 to N-1.
  --simulationLength N          Number of time steps (at least 1), numbered 0 to N-1.

Optional:
  --groundFloorProbability N    Integer percentage from 0 to 100. For passengers
                               above floor 0, choose destination 0 with this
                               probability; otherwise choose another nonzero floor.
                               Omit for uniformly random valid destinations.
                               With two floors, always use the other floor.
  --help                       Show this help.

Counts must be integers no larger than 2147483647.
Output: time,id,source,dest (source and destination are always distinct).

Example:
  bash scripts/generateInputData.sh --nPassengerIds 100 --nFloors 50 \
    --simulationLength 1000 --groundFloorProbability 70 > input.csv
HELP
}

# Report invalid input on stderr, keeping stdout suitable for CSV redirection.
fail() {
    printf 'Error: %s\n' "$1" >&2
    exit 1
}

# Validate before using values in Bash arithmetic; reject leading zeros.
validate_integer() {
    local name=$1 value=$2 minimum=$3 maximum=$4

    if [[ ! $value =~ ^(0|[1-9][0-9]*)$ ]] || ((${#value} > 10)); then
        fail "$name must be an integer between $minimum and $maximum."
    fi
    if ((value < minimum || value > maximum)); then
        fail "$name must be an integer between $minimum and $maximum."
    fi
}

# Draw an integer uniformly from an inclusive range (GNU coreutils).
random_integer() {
    shuf -i "$1-$2" -n 1
}

# Choose a distinct destination, optionally biasing trips towards floor 0.
choose_destination() {
    local source=$1 destination roll

    if ((n_floors == 2)); then
        printf '%s\n' "$((1 - source))"
    elif [[ -n $ground_floor_probability ]] && ((source != 0)); then
        roll=$(random_integer 1 100) || return
        if ((roll <= ground_floor_probability)); then
            printf '0\n'
        else
            # Map a compact range onto nonzero floors, skipping the source.
            destination=$(random_integer 1 "$((n_floors - 2))") || return
            if ((destination >= source)); then
                destination=$((destination + 1))
            fi
            printf '%s\n' "$destination"
        fi
    else
        # Map a compact range onto all floors except the source.
        destination=$(random_integer 0 "$((n_floors - 2))") || return
        if ((destination >= source)); then
            destination=$((destination + 1))
        fi
        printf '%s\n' "$destination"
    fi
}

# Parse arguments without assigning defaults to the required parameters.
n_passengers=''
n_floors=''
simulation_length=''
ground_floor_probability=''

while (($# > 0)); do
    case "$1" in
        --help)
            usage
            exit 0
            ;;
        --nPassengerIds|--nFloors|--simulationLength|--groundFloorProbability)
            (($# >= 2)) || fail "Missing value for $1."
            [[ -n $2 && $2 != --* ]] || fail "Missing value for $1."
            case "$1" in
                --nPassengerIds) n_passengers=$2 ;;
                --nFloors) n_floors=$2 ;;
                --simulationLength) simulation_length=$2 ;;
                --groundFloorProbability) ground_floor_probability=$2 ;;
            esac
            shift 2
            ;;
        *)
            fail "Unknown argument: $1. Use --help for usage."
            ;;
    esac
done

# Check inputs and dependencies before writing any CSV output.
validate_integer --nPassengerIds "$n_passengers" 1 2147483647
validate_integer --nFloors "$n_floors" 2 2147483647
validate_integer --simulationLength "$simulation_length" 1 2147483647
if [[ -n $ground_floor_probability ]]; then
    validate_integer --groundFloorProbability "$ground_floor_probability" 0 100
fi

command -v shuf >/dev/null || fail "Required command not found: shuf (GNU coreutils)."
command -v sort >/dev/null || fail "Required command not found: sort."

# Keep the header first, then sort requests numerically by arrival time.
printf 'time,id,source,dest\n'
for ((passenger = 1; passenger <= n_passengers; passenger++)); do
    time=$(random_integer 0 "$((simulation_length - 1))")
    source=$(random_integer 0 "$((n_floors - 1))")
    destination=$(choose_destination "$source")
    printf '%s,passenger%s,%s,%s\n' "$time" "$passenger" "$source" "$destination"
done | LC_ALL=C sort -s -t ',' -k1,1n
