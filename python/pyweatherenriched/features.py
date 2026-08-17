"""Climate-aware feature engineering on top of PyWeatherEnriched's real
weather data.

Every function here operates on a plain :class:`pandas.DataFrame` using
:mod:`pandas`/:mod:`numpy` and standard, well-established feature-engineering
formulas (rolling aggregates, heating/cooling degree-days, cyclical time
encoding, anomaly-vs-baseline z-scores) — the kind of features a real ML
pipeline would build on top of an hourly/daily weather time series. None of
it is a network call; :func:`enrich_dataframe` is the one function that
bridges to the live Rust core (geocoding + historical weather fetch) to
*produce* the raw weather columns these functions then engineer features
from.
"""

from __future__ import annotations

from typing import Sequence

import numpy as np
import pandas as pd

__all__ = [
    "enrich_dataframe",
    "add_rolling_features",
    "add_degree_days",
    "add_cyclical_time_features",
    "add_anomaly_features",
    "build_features",
]


def enrich_dataframe(
    enricher,
    df: pd.DataFrame,
    location_col: str = "location",
    timestamp_col: str = "timestamp",
) -> pd.DataFrame:
    """Fetch real weather for every (location, timestamp) row in `df`.

    `enricher` is a `pyweatherenriched.WeatherEnricher` instance (backed by
    the compiled Rust core: real Nominatim geocoding + real Open-Meteo
    historical weather, cached in-process). Returns a copy of `df` with
    `latitude`, `longitude`, `temperature`, `humidity`, and `condition`
    columns appended, aligned by row. Rows whose lookup failed (unresolvable
    location, upstream error, ...) get `NaN`/`None` in the new columns
    rather than raising, so one bad row doesn't lose an entire batch.

    Example
    -------
    >>> import pandas as pd
    >>> from pyweatherenriched import WeatherEnricher, enrich_dataframe
    >>> df = pd.DataFrame({
    ...     "order_id": ["A1", "A2"],
    ...     "location": ["Chicago", "Miami"],
    ...     "timestamp": ["2024-06-01T12:00:00", "2024-06-01T12:00:00"],
    ... })
    >>> enriched = enrich_dataframe(WeatherEnricher(), df)  # doctest: +SKIP
    """
    if location_col not in df.columns:
        raise KeyError(f"location_col {location_col!r} not found in DataFrame columns")
    if timestamp_col not in df.columns:
        raise KeyError(f"timestamp_col {timestamp_col!r} not found in DataFrame columns")

    rows = list(
        zip(df[location_col].astype(str), df[timestamp_col].astype(str))
    )
    results = enricher.enrich_batch(rows)

    latitude = [r.get("latitude") for r in results]
    longitude = [r.get("longitude") for r in results]
    temperature = [r.get("temperature") for r in results]
    humidity = [r.get("humidity") for r in results]
    condition = [r.get("condition") for r in results]

    out = df.copy()
    out["latitude"] = pd.Series(latitude, index=out.index, dtype="float64")
    out["longitude"] = pd.Series(longitude, index=out.index, dtype="float64")
    out["temperature"] = pd.Series(temperature, index=out.index, dtype="float64")
    out["humidity"] = pd.Series(humidity, index=out.index, dtype="float64")
    out["condition"] = pd.Series(condition, index=out.index, dtype="object")
    return out


def add_rolling_features(
    df: pd.DataFrame,
    value_col: str,
    window: int,
    group_col: str | None = None,
    stats: Sequence[str] = ("mean", "std", "min", "max"),
    min_periods: int = 1,
) -> pd.DataFrame:
    """Add rolling-window aggregates of `value_col` (e.g. a `window`-row
    trailing average of temperature).

    `df` is assumed to already be sorted by time (ascending) within each
    group — the same ordering the rolling window operates over. If
    `group_col` is given (e.g. a location or sensor ID), rolling windows are
    computed independently per group so one location's history never leaks
    into another's.

    Adds one column per stat: `f"{value_col}_roll{window}_{stat}"`.
    """
    if value_col not in df.columns:
        raise KeyError(f"value_col {value_col!r} not found in DataFrame columns")
    if window < 1:
        raise ValueError(f"window must be >= 1, got {window}")

    out = df.copy()
    if group_col is not None:
        if group_col not in df.columns:
            raise KeyError(f"group_col {group_col!r} not found in DataFrame columns")
        roller = out.groupby(group_col)[value_col].rolling(
            window=window, min_periods=min_periods
        )
    else:
        roller = out[value_col].rolling(window=window, min_periods=min_periods)

    for stat in stats:
        if not hasattr(roller, stat):
            raise ValueError(f"unsupported rolling stat: {stat!r}")
        series = getattr(roller, stat)()
        if group_col is not None:
            # groupby().rolling() returns a MultiIndex (group, original
            # index); drop the group level to realign with `out`.
            series = series.reset_index(level=0, drop=True)
        out[f"{value_col}_roll{window}_{stat}"] = series

    return out


def add_degree_days(
    df: pd.DataFrame,
    temp_col: str = "temperature",
    base_temp: float = 18.0,
) -> pd.DataFrame:
    """Add heating-degree-day (HDD) and cooling-degree-day (CDD) columns.

    Standard degree-day definitions relative to a base temperature
    (default 18°C / 65°F, the traditional HVAC balance point):

    - HDD = max(0, base_temp - temperature)  — heating demand signal
    - CDD = max(0, temperature - base_temp)  — cooling demand signal

    Applied per-row here (e.g. per hourly reading); sum/mean HDD or CDD over
    a day/month for the classic degree-day aggregate used in energy-demand
    and agriculture models.
    """
    if temp_col not in df.columns:
        raise KeyError(f"temp_col {temp_col!r} not found in DataFrame columns")

    out = df.copy()
    temp = out[temp_col].astype(float)
    out["hdd"] = np.maximum(0.0, base_temp - temp)
    out["cdd"] = np.maximum(0.0, temp - base_temp)
    return out


def add_cyclical_time_features(
    df: pd.DataFrame,
    timestamp_col: str = "timestamp",
) -> pd.DataFrame:
    """Add sin/cos-encoded (cyclical) hour-of-day, day-of-week, and
    day-of-year features.

    Raw integer month/hour/day-of-year values imply a false discontinuity
    (hour 23 and hour 0 are adjacent in reality but 23 apart numerically).
    Sin/cos encoding onto the unit circle preserves that adjacency, which is
    standard practice for feeding cyclical time features into ML models.
    """
    if timestamp_col not in df.columns:
        raise KeyError(f"timestamp_col {timestamp_col!r} not found in DataFrame columns")

    out = df.copy()
    ts = pd.to_datetime(out[timestamp_col], utc=False, errors="coerce")

    hour = ts.dt.hour + ts.dt.minute / 60.0
    out["hour_sin"] = np.sin(2 * np.pi * hour / 24.0)
    out["hour_cos"] = np.cos(2 * np.pi * hour / 24.0)

    dow = ts.dt.dayofweek  # Monday=0
    out["day_of_week_sin"] = np.sin(2 * np.pi * dow / 7.0)
    out["day_of_week_cos"] = np.cos(2 * np.pi * dow / 7.0)

    doy = ts.dt.dayofyear
    days_in_year = np.where(ts.dt.is_leap_year, 366.0, 365.0)
    out["day_of_year_sin"] = np.sin(2 * np.pi * doy / days_in_year)
    out["day_of_year_cos"] = np.cos(2 * np.pi * doy / days_in_year)

    return out


def add_anomaly_features(
    df: pd.DataFrame,
    value_col: str,
    group_col: str | None = None,
    baseline: str = "expanding",
    window: int | None = None,
) -> pd.DataFrame:
    """Add a z-score column measuring how anomalous each `value_col`
    reading is relative to a baseline mean/std.

    `baseline` selects how the mean/std are computed:
    - `"expanding"` (default): mean/std of all *prior* values up to and
      including the current row (a growing baseline — good when you want
      "how unusual is this compared to everything we've seen so far").
    - `"rolling"`: mean/std over the trailing `window` rows (requires
      `window`) — good for a baseline that adapts to recent conditions
      (e.g. a seasonal drift) rather than all of history.
    - `"global"`: a single mean/std computed over the whole column (or
      per-group, if `group_col` is set) — good for "how unusual is this
      compared to the dataset as a whole".

    Adds `f"{value_col}_zscore"`; rows where the baseline std is 0 or
    undefined (e.g. the very first row of an expanding baseline) get `NaN`
    rather than a division-by-zero artifact.
    """
    if value_col not in df.columns:
        raise KeyError(f"value_col {value_col!r} not found in DataFrame columns")
    if baseline == "rolling" and not window:
        raise ValueError("baseline='rolling' requires a positive `window`")

    out = df.copy()

    def _zscore(series: pd.Series) -> pd.Series:
        series = series.astype(float)
        if baseline == "expanding":
            mean = series.expanding(min_periods=1).mean()
            std = series.expanding(min_periods=2).std()
        elif baseline == "rolling":
            mean = series.rolling(window=window, min_periods=1).mean()
            std = series.rolling(window=window, min_periods=2).std()
        elif baseline == "global":
            mean = pd.Series(series.mean(), index=series.index)
            std = pd.Series(series.std(), index=series.index)
        else:
            raise ValueError(f"unsupported baseline: {baseline!r}")
        with np.errstate(invalid="ignore", divide="ignore"):
            z = (series - mean) / std
        return z.replace([np.inf, -np.inf], np.nan)

    if group_col is not None:
        if group_col not in df.columns:
            raise KeyError(f"group_col {group_col!r} not found in DataFrame columns")
        out[f"{value_col}_zscore"] = (
            out.groupby(group_col)[value_col].apply(_zscore).reset_index(level=0, drop=True)
        )
    else:
        out[f"{value_col}_zscore"] = _zscore(out[value_col])

    return out


def build_features(
    df: pd.DataFrame,
    temp_col: str = "temperature",
    timestamp_col: str = "timestamp",
    group_col: str | None = None,
    base_temp: float = 18.0,
    rolling_window: int = 24,
    rolling_stats: Sequence[str] = ("mean", "std"),
) -> pd.DataFrame:
    """Convenience pipeline: degree-days + cyclical time features + a
    rolling window over `temp_col` + an anomaly z-score, in one call.

    Equivalent to calling `add_degree_days`, `add_cyclical_time_features`,
    `add_rolling_features`, and `add_anomaly_features` in sequence with
    sensible defaults; use the individual functions directly for more
    control over any one step.
    """
    out = add_degree_days(df, temp_col=temp_col, base_temp=base_temp)
    out = add_cyclical_time_features(out, timestamp_col=timestamp_col)
    out = add_rolling_features(
        out, value_col=temp_col, window=rolling_window, group_col=group_col, stats=rolling_stats
    )
    out = add_anomaly_features(out, value_col=temp_col, group_col=group_col, baseline="expanding")
    return out
