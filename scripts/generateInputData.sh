#!/usr/bin/env bash
set -euo pipefail

# Generate chronological requests from a fixed pool of passenger IDs.

# Describe the required parameters and the optional destination weighting.
usage() {
    cat <<'HELP'
Usage:
  generateInputData.sh --nPassengerIds N --nFloors N --simulationLength N
                       [--groundFloorProbability PERCENT]
                       [--requestProbability PROBABILITY]
                       [--busyPeriod START:END:MULTIPLIER ...]

Required:
  --nPassengerIds N             Size of the passenger ID pool (at least 1).
  --nFloors N                   Number of floors (at least 2), numbered 0 to N-1.
  --simulationLength N          Number of time steps (at least 1), numbered 0 to N-1.

Optional:
  --groundFloorProbability N    Integer percentage from 0 to 100. For passengers
                               above floor 0, choose destination 0 with this
                               probability; otherwise choose another nonzero floor.
                               Omit for uniformly random valid destinations.
                               With two floors, always use the other floor.
  --requestProbability P       Probability per request opportunity (default 0.25).
                               Between 0 and 1, with up to four decimal places.
                               Normally one opportunity per epoch; M during an
                               M-times busy period. Passengers have a 10-epoch
                               cooldown. Skip requests when none are available.
  --busyPeriod START:END:M      Optional, repeatable busy interval [START, END).
                               Each time step has M times normal demand.
                               M must be a positive integer; intervals must be
                               within the simulation and must not overlap.
                               Adds opportunities, not new passenger IDs.
                               Omit for one opportunity per epoch.
  --help                       Show this help.

Counts must be integers no larger than 2147483647.
Output: time,id,source,dest (source and destination are always distinct).
IDs may repeat; not every passenger is guaranteed to make a request.

Example:
  bash scripts/generateInputData.sh --nPassengerIds 100 --nFloors 50 \
    --simulationLength 1000 --groundFloorProbability 70 > input.csv

Busy-period example:
  bash scripts/generateInputData.sh --nPassengerIds 100 --nFloors 50 \
    --simulationLength 1000 --busyPeriod 100:200:4 \
    --busyPeriod 700:800:2 > input.csv
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

# Convert a decimal probability to an integer threshold out of 10000.
prepare_request_probability() {
    local fraction

    [[ $request_probability =~ ^(0(\.[0-9]{1,4})?|1(\.0{1,4})?)$ ]] ||
        fail "--requestProbability must be between 0 and 1, with up to four decimal places."
    if [[ $request_probability == 1* ]]; then
        request_threshold=10000
    else
        fraction=${request_probability#0}
        fraction=${fraction#.}
        fraction="${fraction}0000"
        request_threshold=$((10#${fraction:0:4}))
    fi
}

# Busy periods: validate intervals once, independently of request generation.
prepare_busy_periods() {
    local period start end multiplier previous

    for period in "${busy_periods[@]}"; do
        [[ $period =~ ^[0-9]+:[0-9]+:[0-9]+$ ]] ||
            fail "--busyPeriod must have format START:END:MULTIPLIER."
        IFS=: read -r start end multiplier <<< "$period"
        validate_integer "Busy period start" "$start" 0 "$((simulation_length - 1))"
        validate_integer "Busy period end" "$end" 1 "$simulation_length"
        validate_integer "Busy period multiplier" "$multiplier" 1 2147483647
        ((start < end)) || fail "Busy period start must be less than end."

        for previous in "${!busy_starts[@]}"; do
            if ((start < busy_ends[previous] && end > busy_starts[previous])); then
                fail "Busy periods must not overlap."
            fi
        done

        busy_starts+=("$start")
        busy_ends+=("$end")
        busy_multipliers+=("$multiplier")
    done
}

# Busy periods: return the number of independent opportunities in this epoch.
request_opportunities() {
    local time=$1 period
    for period in "${!busy_starts[@]}"; do
        if ((time >= busy_starts[period] && time < busy_ends[period])); then
            printf '%s\n' "${busy_multipliers[period]}"
            return
        fi
    done
    printf '1\n'
}

# Choose a distinct destination, optionally biasing trips towards floor 0.
choose_destination() {
    local source=$1 destination roll

    if [[ -n $ground_floor_probability ]] && ((source != 0 && n_floors > 2)); then
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
request_probability=0.25
passenger_cooldown=10

# Busy-period state is independent of floor and passenger generation.
busy_periods=()
busy_starts=()
busy_ends=()
busy_multipliers=()

while (($# > 0)); do
    case "$1" in
        --help)
            usage
            exit 0
            ;;
        --nPassengerIds|--nFloors|--simulationLength|--groundFloorProbability|--busyPeriod|--requestProbability)
            (($# >= 2)) || fail "Missing value for $1."
            [[ -n $2 && $2 != --* ]] || fail "Missing value for $1."
            case "$1" in
                --nPassengerIds) n_passengers=$2 ;;
                --nFloors) n_floors=$2 ;;
                --simulationLength) simulation_length=$2 ;;
                --groundFloorProbability) ground_floor_probability=$2 ;;
                --busyPeriod) busy_periods+=("$2") ;;
                --requestProbability) request_probability=$2 ;;
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

prepare_busy_periods
prepare_request_probability

command -v shuf >/dev/null || fail "Required command not found: shuf (GNU coreutils)."

# Advance time, rebuilding the eligible pool as passenger cooldowns expire.
# Choosing directly from this pool avoids endlessly redrawing unavailable IDs.
last_request_times=()
printf 'time,id,source,dest\n'
for ((time = 0; time < simulation_length; time++)); do
    eligible_passengers=()
    for ((passenger = 1; passenger <= n_passengers; passenger++)); do
        last_request=${last_request_times[passenger]:--$passenger_cooldown}
        if ((time - last_request >= passenger_cooldown)); then
            eligible_passengers+=("$passenger")
        fi
    done

    opportunities=$(request_opportunities "$time")
    for ((opportunity = 0; opportunity < opportunities; opportunity++)); do
        ((${#eligible_passengers[@]} > 0)) || break
        roll=$(random_integer 1 10000)
        ((roll <= request_threshold)) || continue

        index=$(random_integer 0 "$((${#eligible_passengers[@]} - 1))")
        passenger=${eligible_passengers[index]}
        source=$(random_integer 0 "$((n_floors - 1))")
        destination=$(choose_destination "$source")
        printf '%s,passenger%s,%s,%s\n' "$time" "$passenger" "$source" "$destination"
        last_request_times[passenger]=$time

        # Remove this passenger from this epoch's pool without leaving gaps.
        last_index=$((${#eligible_passengers[@]} - 1))
        eligible_passengers[index]=${eligible_passengers[last_index]}
        unset 'eligible_passengers[last_index]'
    done
done
