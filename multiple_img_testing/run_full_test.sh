#!/bin/bash
# Master script to run complete multi-image stress test

echo "╔════════════════════════════════════════════════════════════════════════════╗"
echo "║  MULTI-IMAGE STRESS TEST - COMPLETE TEST SUITE                            ║"
echo "╚════════════════════════════════════════════════════════════════════════════╝"
echo ""

# Check if Python is available
if ! command -v python3 &> /dev/null; then
    echo "❌ Python 3 is required but not found"
    exit 1
fi

# Check if config exists
if [ ! -f "config.json" ]; then
    echo "❌ config.json not found"
    exit 1
fi

echo "Step 1: Cleaning previous results..."
rm -rf output/encrypted/* output/decrypted/* output/test_images/* output/metrics/*
echo "✓ Cleaned output directories"
echo ""

echo "Step 2: Running multi-process stress test..."
echo "════════════════════════════════════════════════════════════════════════════"
python3 test_coordinator.py

if [ $? -ne 0 ]; then
    echo ""
    echo "❌ Stress test failed or had errors"
    echo "Check output/metrics/ for details"
    exit 1
fi

echo ""
echo "Step 3: Decrypting all encrypted images..."
echo "════════════════════════════════════════════════════════════════════════════"
python3 decrypt_all.py

if [ $? -ne 0 ]; then
    echo ""
    echo "⚠️  Some images failed to decrypt"
    echo "Check output/metrics/decryption_results.json for details"
fi

echo ""
echo "Step 4: Analyzing metrics and generating report..."
echo "════════════════════════════════════════════════════════════════════════════"
python3 analyze_metrics.py

echo ""
echo "╔════════════════════════════════════════════════════════════════════════════╗"
echo "║  TEST COMPLETE                                                             ║"
echo "╚════════════════════════════════════════════════════════════════════════════╝"
echo ""
echo "📁 Results saved to:"
echo "   - output/test_images/    - Generated test images"
echo "   - output/encrypted/      - Encrypted images from servers"
echo "   - output/decrypted/      - Decrypted images (verification)"
echo "   - output/metrics/        - Metrics, analysis, and plots"
echo ""
echo "📊 View the analysis report:"
echo "   cat output/metrics/analysis_report.txt"
echo ""
echo "📈 View detailed metrics:"
echo "   cat output/metrics/aggregated_metrics.json"
echo ""
