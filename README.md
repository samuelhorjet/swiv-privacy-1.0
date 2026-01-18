# Pred Markets

## --- ACCURACY SCORE (Linear Normalization) ---

Formula: Accuracy = 1.0 - ( |Prediction - Result| / Buffer )
Returns a value between 0 and MATH_PRECISION (0.0 to 1.0)

Fraction of error = diff / buffer
Score = 1.0 - Fraction

## --- TIME BONUS (Linear Decay) ---
/// Formula: Factor = 1.0 + ( (EndTime - EntryTime) / TotalDuration )
/// - Entry at Start: Bonus = 1.0 + 1.0 = 2.0x
/// - Entry at End: Bonus = 1.0 + 0.0 = 1.0x