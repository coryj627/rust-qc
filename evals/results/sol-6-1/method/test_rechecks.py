#!/usr/bin/env python3
"""Keep the random recheck sample reproducible without filling empty populations."""

import unittest

from prepare_rechecks import sample_scores


class RecheckTests(unittest.TestCase):
    def test_sample_is_ceiling_ten_percent_without_replacement(self):
        population = list(range(56))
        sample = sample_scores(population, 123)
        self.assertEqual(len(sample), 6)
        self.assertEqual(len(set(sample)), 6)
        self.assertEqual(sample, sample_scores(population, 123))

    def test_no_scores_produce_no_recheck_instead_of_a_fake_observation(self):
        self.assertEqual(sample_scores([], 123), [])


if __name__ == "__main__":
    unittest.main()
