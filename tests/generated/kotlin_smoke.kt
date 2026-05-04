// Copyright (c) Meta Platforms, Inc. and affiliates.

fun main() {
    val args =
        AgentRunArgs(
            task = "summarize",
            workspace = "/repo",
            verbose = 2,
            model = AgentRunModel.Frontier,
            temperature = 0.2,
            maxTokens = 2048,
            env = listOf("RUST_LOG=info"),
            dryRun = true,
        )

    val argv = buildAgentRunCommand(args)
    check(
        argv ==
            listOf(
                "--workspace",
                "/repo",
                "--verbose",
                "--verbose",
                "agent",
                "run",
                "summarize",
                "--model",
                "frontier",
                "--temperature",
                "0.2",
                "--max-tokens",
                "2048",
                "--env",
                "RUST_LOG=info",
                "--dry-run",
            ),
    )
}
