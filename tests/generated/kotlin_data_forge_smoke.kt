// Copyright (c) Meta Platforms, Inc. and affiliates.

fun main() {
    val args =
        DatasetImportArgs(
            source = "data/input.csv",
            format = DatasetImportFormat.Csv,
            output = "target/out",
            logLevel = DatasetImportLogLevel.Debug,
            noColor = true,
            schema = "schema.json",
            sampleRate = 0.25,
            tag = listOf("raw", "daily"),
        )

    val argv = buildDatasetImportCommand(args)
    check(
        argv ==
            listOf(
                "--output",
                "target/out",
                "--log-level",
                "debug",
                "--no-color",
                "dataset",
                "import",
                "--source",
                "data/input.csv",
                "--format",
                "csv",
                "--schema",
                "schema.json",
                "--sample-rate",
                "0.25",
                "--tag",
                "raw",
                "--tag",
                "daily",
            ),
    )
}
