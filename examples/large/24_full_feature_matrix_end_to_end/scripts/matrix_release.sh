# Example: large/24_full_feature_matrix_end_to_end
# File: scripts/matrix_release.sh
# Scenario: full feature matrix end to end

mkdir -p out
cat \
  out/full-bootstrap.txt \
  out/full-seed.txt \
  out/full-common-lint.txt \
  out/full-qa-validate.txt \
  > out/full_matrix_release.txt
echo matrix-release >> out/full_matrix_release.txt
