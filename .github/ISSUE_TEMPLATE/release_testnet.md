---
name: Testnet release
about: Checklist for releasing a testnet
title: ''
labels: ''
assignees: ''

---

# Testnet Release

Testnet name: X
Release date: X
Testnet release manager: X

# Testnet Release Manager Checklist

Preceding Friday (sprint planning day):

- [ ] Create GitHub project column, work with team to populate the milestone with tickets targeted for the release.

Tuesday (or after release of previous testnet):

- [ ] Construct the genesis data for the release:
  - [ ] Create new testnet directory with initial genesis allocations for this testnet by running `cd testnets && ./new-testnet.sh`
    - This genesis data will be used for `testnet-preview` with a randomized version of the future testnet's chain ID.

Thursday:

- [ ] Check in with team again in a release meeting and update the GitHub milestone to ensure it represents what will make it into the testnet.
- [ ] Draft an announcement for peer review to ensure major changes included are comprehensive.

Following Monday (release day):

- [ ] Verify that `testnet-preview.penumbra.zone` is operational; it is redeployed on every push to main, and is an exact preview of what is about to be deployed.
- [ ] Bump the version number and push its tag, via [cargo-release](https://crates.io/crates/cargo-release).
    - [ ] Run `cargo release minor` for a new testnet, or `cargo release patch` for a bugfix. For the latter, make sure you're on a dedicated release branch.
    - [ ] Push the commit and newly generated tag, e.g. `v0.51.0`, to the remote.
- [ ] Wait for the "Release" workflow to complete: it'll take ~90m, most of which is the macOS builds.
- [ ] Edit the newly created (and published) release object, then click "Generate release notes." Cut and paste the generated text from the bottom to the top of the post, then save it.
- [ ] You must [manually review](https://docs.github.com/en/actions/managing-workflow-runs/reviewing-deployments) the `Waiting` deployment in the GitHub Action UI before the deployment will begin. Monitor the GitHub action to ensure it completes after it is approved.
- [ ] Delegate to the Penumbra Labs CI validators; use amounts of ~200k `penumbra` per validator.
- [ ] Update Galileo deployment, [following docs](https://github.com/penumbra-zone/galileo).
- [ ] Make GitHub release object and include the announcement
- [ ] Make the announcement to Discord! 🎉🎉🎉
