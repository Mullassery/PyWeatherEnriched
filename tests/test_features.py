"""Tests for pyweatherenriched.features — pandas/numpy climate feature
engineering. All synthetic (no network); test_python_bindings.py covers the
live enrich_dataframe path.
"""

import numpy as np
import pandas as pd
import pytest

from pyweatherenriched import features


class TestAddRollingFeatures:
    def test_rolling_mean_matches_pandas_directly(self):
        df = pd.DataFrame({"temperature": [10.0, 20.0, 30.0, 40.0, 50.0]})
        out = features.add_rolling_features(df, "temperature", window=2, stats=("mean",))

        expected = df["temperature"].rolling(window=2, min_periods=1).mean()
        pd.testing.assert_series_equal(
            out["temperature_roll2_mean"], expected.rename("temperature_roll2_mean")
        )

    def test_rolling_respects_groups_independently(self):
        df = pd.DataFrame(
            {
                "location": ["A", "A", "A", "B", "B", "B"],
                "temperature": [10.0, 20.0, 30.0, 100.0, 200.0, 300.0],
            }
        )
        out = features.add_rolling_features(
            df, "temperature", window=2, group_col="location", stats=("mean",)
        )
        # Group B's rolling mean must never be polluted by group A's values.
        assert out.loc[3, "temperature_roll2_mean"] == 100.0  # first B row: only itself
        assert out.loc[4, "temperature_roll2_mean"] == 150.0  # mean(100, 200)
        assert out.loc[5, "temperature_roll2_mean"] == 250.0  # mean(200, 300)

    def test_unknown_stat_raises(self):
        df = pd.DataFrame({"temperature": [1.0, 2.0]})
        with pytest.raises(ValueError):
            features.add_rolling_features(df, "temperature", window=2, stats=("not_a_stat",))

    def test_missing_column_raises_key_error(self):
        df = pd.DataFrame({"other": [1.0]})
        with pytest.raises(KeyError):
            features.add_rolling_features(df, "temperature", window=2)


class TestAddDegreeDays:
    def test_hdd_and_cdd_are_mutually_exclusive_and_correct(self):
        df = pd.DataFrame({"temperature": [0.0, 18.0, 30.0]})
        out = features.add_degree_days(df, base_temp=18.0)

        assert out["hdd"].tolist() == [18.0, 0.0, 0.0]
        assert out["cdd"].tolist() == [0.0, 0.0, 12.0]

    def test_never_negative(self):
        df = pd.DataFrame({"temperature": np.linspace(-40, 40, 20)})
        out = features.add_degree_days(df)
        assert (out["hdd"] >= 0).all()
        assert (out["cdd"] >= 0).all()

    def test_custom_base_temp(self):
        df = pd.DataFrame({"temperature": [65.0]})
        out = features.add_degree_days(df, base_temp=65.0)
        assert out["hdd"].iloc[0] == 0.0
        assert out["cdd"].iloc[0] == 0.0


class TestAddCyclicalTimeFeatures:
    def test_midnight_and_almost_midnight_are_close_on_the_circle(self):
        # This is the entire point of cyclical encoding: 23:59 and 00:00
        # are 1 minute apart in reality, not ~24 hours apart.
        df = pd.DataFrame(
            {"timestamp": ["2024-01-01T00:00:00", "2024-01-01T23:59:00"]}
        )
        out = features.add_cyclical_time_features(df)
        dist = np.hypot(
            out["hour_sin"].iloc[0] - out["hour_sin"].iloc[1],
            out["hour_cos"].iloc[0] - out["hour_cos"].iloc[1],
        )
        assert dist < 0.01

    def test_unit_circle_bounds(self):
        df = pd.DataFrame(
            {
                "timestamp": pd.date_range("2024-01-01", periods=50, freq="7h").astype(
                    str
                )
            }
        )
        out = features.add_cyclical_time_features(df)
        for col in ["hour_sin", "hour_cos", "day_of_week_sin", "day_of_week_cos", "day_of_year_sin", "day_of_year_cos"]:
            assert out[col].between(-1.0001, 1.0001).all()

    def test_noon_and_midnight_are_opposite_on_hour_circle(self):
        df = pd.DataFrame({"timestamp": ["2024-01-01T00:00:00", "2024-01-01T12:00:00"]})
        out = features.add_cyclical_time_features(df)
        assert out["hour_sin"].iloc[0] == pytest.approx(0.0, abs=1e-9)
        assert out["hour_cos"].iloc[0] == pytest.approx(1.0, abs=1e-9)
        assert out["hour_sin"].iloc[1] == pytest.approx(0.0, abs=1e-9)
        assert out["hour_cos"].iloc[1] == pytest.approx(-1.0, abs=1e-9)


class TestAddAnomalyFeatures:
    def test_constant_series_has_no_anomaly(self):
        df = pd.DataFrame({"temperature": [20.0] * 10})
        out = features.add_anomaly_features(df, "temperature", baseline="global")
        # constant series -> std 0 -> z-score defined as NaN, not inf/garbage
        assert out["temperature_zscore"].isna().all()

    def test_obvious_spike_has_large_zscore(self):
        df = pd.DataFrame({"temperature": [20.0] * 20 + [200.0]})
        out = features.add_anomaly_features(df, "temperature", baseline="global")
        assert abs(out["temperature_zscore"].iloc[-1]) > 3

    def test_rolling_baseline_requires_window(self):
        df = pd.DataFrame({"temperature": [1.0, 2.0]})
        with pytest.raises(ValueError):
            features.add_anomaly_features(df, "temperature", baseline="rolling")

    def test_expanding_first_row_is_nan_not_divide_by_zero_error(self):
        df = pd.DataFrame({"temperature": [10.0, 20.0, 30.0]})
        out = features.add_anomaly_features(df, "temperature", baseline="expanding")
        assert pd.isna(out["temperature_zscore"].iloc[0])

    def test_groups_do_not_leak_into_each_others_baseline(self):
        df = pd.DataFrame(
            {
                "location": ["A"] * 5 + ["B"] * 5,
                "temperature": [20.0] * 4 + [200.0] + [-50.0] * 5,
            }
        )
        out = features.add_anomaly_features(
            df, "temperature", group_col="location", baseline="global"
        )
        # Group B is constant -> NaN, regardless of group A's huge spike.
        assert out.loc[out["location"] == "B", "temperature_zscore"].isna().all()


class TestBuildFeaturesPipeline:
    def test_pipeline_adds_all_expected_columns(self):
        df = pd.DataFrame(
            {
                "temperature": [10.0, 15.0, 5.0, 20.0],
                "timestamp": pd.date_range("2024-01-01", periods=4, freq="6h").astype(str),
            }
        )
        out = features.build_features(df, rolling_window=2)
        for col in [
            "hdd",
            "cdd",
            "hour_sin",
            "hour_cos",
            "day_of_week_sin",
            "day_of_week_cos",
            "temperature_roll2_mean",
            "temperature_roll2_std",
            "temperature_zscore",
        ]:
            assert col in out.columns
        assert len(out) == len(df)


class TestEnrichDataframe:
    def test_missing_location_column_raises(self):
        df = pd.DataFrame({"timestamp": ["2024-01-01T00:00:00"]})

        class _StubEnricher:
            def enrich_batch(self, rows):
                return []

        with pytest.raises(KeyError):
            features.enrich_dataframe(_StubEnricher(), df)

    def test_uses_enrich_batch_and_aligns_columns_by_row(self):
        calls = {}

        class _StubEnricher:
            def enrich_batch(self, rows):
                calls["rows"] = rows
                return [
                    {
                        "latitude": 1.0,
                        "longitude": 2.0,
                        "temperature": 10.0,
                        "humidity": 50.0,
                        "condition": "Clear",
                    },
                    {"error": "Enrichment failed"},
                ]

        df = pd.DataFrame(
            {
                "location": ["Nowhere1", "Nowhere2"],
                "timestamp": ["2024-01-01T00:00:00", "2024-01-02T00:00:00"],
            }
        )
        out = features.enrich_dataframe(_StubEnricher(), df)

        assert calls["rows"] == [
            ("Nowhere1", "2024-01-01T00:00:00"),
            ("Nowhere2", "2024-01-02T00:00:00"),
        ]
        assert out.loc[0, "temperature"] == 10.0
        assert pd.isna(out.loc[1, "temperature"])  # failed row -> NaN, not fabricated data
