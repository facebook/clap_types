// Copyright (c) Meta Platforms, Inc. and affiliates.

fun main() {
  val args =
      DeployArgs(
          service = "api",
          image = "registry.example.com/api:1",
          workspace = "/workspace",
          output = DeployOutput.Json,
          verbose = 1,
          environment = DeployEnvironment.Prod,
          replicas = 3,
          env = listOf("RUST_LOG=info"),
          wait = true,
          timeoutSeconds = 30.5,
      )

  val argv = buildDeployCommand(args)
  check(
      argv ==
          listOf(
              "--workspace",
              "/workspace",
              "--output",
              "json",
              "--verbose",
              "deploy",
              "api",
              "--image",
              "registry.example.com/api:1",
              "--environment",
              "prod",
              "--replicas",
              "3",
              "--env",
              "RUST_LOG=info",
              "--wait",
              "--timeout-seconds",
              "30.5",
          ),
  )
}
