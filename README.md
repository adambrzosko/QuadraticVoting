# Quadratic voting pallet

Create using a FRAME-based [Substrate](https://www.substrate.io/) node template.

This substrate node template was edited to include a custom pallet [in the `pallets`](./pallets/template/src/lib.rs) directory. The pallet implements a round-based quadratic voting scheme. A user can begin a round at will, becoming this round's admin. During the round, any user can register a proposal (a custom name) and any user can vote on it with an amount of token. The value of the vote is the square root of the amount of the token. The user who started the round can then end it at any time and calculate the votes for all proposals registered.

# Code
Four functions can be called by users - start-round, end-round (only by round admin), create-proposal and vote.

All helper private functions are for getting round and proposal numbers (to avoid duplicates and clashes), creating a voting key, making sure if proposal is registered, calculating votes (vote getting, summing value for a proposal, tallying for all proposals).

Overflows in names, proposal or vote numbers are dealt with bounded vectors. 

# Testing
The pallet can be tested by running
```sh
cargo test
```
in the [`template`](./pallets/template/) directory. There are three tests: for starting and ending rounds, creating a proposal and voting.


# Outlook
More tests can be written to test edge cases and intended functionality. The round creation could be changed so that they are fixed time. Projects could include more features than just a name. The tokens should not just be burned but distributed properly - to the proposal owner, admin or the voters in some scheme.  
