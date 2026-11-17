# Pizza Converter Test Runner PowerShell Wrapper
param(
    [int]$Tier = 0,
    [switch]$Verbose = $false
)

$testsDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$runnerPath = Join-Path $testsDir "run_all_tests.py"

$pythonCmd = "python"
if (-not (Get-Command $pythonCmd -ErrorAction SilentlyContinue)) {
    Write-Error "Python 3 is required to run the test suite."
    exit 1
}

$argsList = @($runnerPath)
if ($Tier -gt 0) {
    $argsList += "--tier"
    $argsList += $Tier
}
if ($Verbose) {
    $argsList += "-v"
}

& $pythonCmd $argsList
exit $LASTEXITCODE
