localFlake:

{ ... }:
{
  # pass `topInputs` to flake level modules
  _module.args.topInputs = localFlake.inputs;
  perSystem = { ... }:
    {
      # pass `topInputs` to `perSystem` level modules
      _module.args.topInputs = localFlake.inputs;
    };
}
