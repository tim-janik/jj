// Copyright 2021 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use jujutsu_lib::commit_builder::CommitBuilder;
use jujutsu_lib::op_store::RefTarget;
use jujutsu_lib::repo_path::RepoPath;
use jujutsu_lib::rewrite::DescendantRebaser;
use jujutsu_lib::testutils;
use jujutsu_lib::testutils::{assert_rebased, CommitGraphBuilder};
use maplit::{hashmap, hashset};
use test_case::test_case;

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_sideways(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit B was replaced by commit F. Commits C-E should be rebased.
    //
    // F
    // | D
    // | C E
    // | |/
    // | B
    // |/
    // A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_d = graph_builder.commit_with_parents(&[&commit_c]);
    let commit_e = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_f = graph_builder.commit_with_parents(&[&commit_a]);

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {
            commit_b.id().clone() => hashset!{commit_f.id().clone()}
        },
        hashset! {},
    );
    let new_commit_c = assert_rebased(rebaser.rebase_next(), &commit_c, &[&commit_f]);
    assert_rebased(rebaser.rebase_next(), &commit_d, &[&new_commit_c]);
    assert_rebased(rebaser.rebase_next(), &commit_e, &[&commit_f]);
    assert!(rebaser.rebase_next().is_none());
    assert_eq!(rebaser.rebased().len(), 3);

    tx.discard();
}

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_forward(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit B was replaced by commit F. Commits C and E should be rebased onto F.
    // Commit D does not get rebased because it's an ancestor of the
    // destination. Commit G does not get replaced because it's already in
    // place.
    //
    // G
    // F E
    // |/
    // D C
    // |/
    // B
    // A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_d = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_e = graph_builder.commit_with_parents(&[&commit_d]);
    let commit_f = graph_builder.commit_with_parents(&[&commit_d]);
    let _commit_g = graph_builder.commit_with_parents(&[&commit_f]);

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {
            commit_b.id().clone() => hashset!{commit_f.id().clone()}
        },
        hashset! {},
    );
    assert_rebased(rebaser.rebase_next(), &commit_c, &[&commit_f]);
    assert_rebased(rebaser.rebase_next(), &commit_e, &[&commit_f]);
    assert!(rebaser.rebase_next().is_none());
    assert_eq!(rebaser.rebased().len(), 2);

    tx.discard();
}

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_backward(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit C was replaced by commit B. Commit D should be rebased.
    //
    // D
    // C
    // B
    // A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_d = graph_builder.commit_with_parents(&[&commit_c]);

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {
            commit_c.id().clone() => hashset!{commit_b.id().clone()}
        },
        hashset! {},
    );
    assert_rebased(rebaser.rebase_next(), &commit_d, &[&commit_b]);
    assert!(rebaser.rebase_next().is_none());
    assert_eq!(rebaser.rebased().len(), 1);

    tx.discard();
}

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_internal_merge(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit B was replaced by commit F. Commits C-E should be rebased.
    //
    // F
    // | E
    // | |\
    // | C D
    // | |/
    // | B
    // |/
    // A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_d = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_e = graph_builder.commit_with_parents(&[&commit_c, &commit_d]);
    let commit_f = graph_builder.commit_with_parents(&[&commit_a]);

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {
            commit_b.id().clone() => hashset!{commit_f.id().clone()}
        },
        hashset! {},
    );
    let new_commit_c = assert_rebased(rebaser.rebase_next(), &commit_c, &[&commit_f]);
    let new_commit_d = assert_rebased(rebaser.rebase_next(), &commit_d, &[&commit_f]);
    assert_rebased(
        rebaser.rebase_next(),
        &commit_e,
        &[&new_commit_c, &new_commit_d],
    );
    assert!(rebaser.rebase_next().is_none());
    assert_eq!(rebaser.rebased().len(), 3);

    tx.discard();
}

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_external_merge(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit C was replaced by commit F. Commits E should be rebased. The rebased
    // commit E should have F as first parent and commit D as second parent.
    //
    // F
    // | E
    // | |\
    // | C D
    // | |/
    // | B
    // |/
    // A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_d = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_e = graph_builder.commit_with_parents(&[&commit_c, &commit_d]);
    let commit_f = graph_builder.commit_with_parents(&[&commit_a]);

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {
            commit_c.id().clone() => hashset!{commit_f.id().clone()}
        },
        hashset! {},
    );
    assert_rebased(rebaser.rebase_next(), &commit_e, &[&commit_f, &commit_d]);
    assert!(rebaser.rebase_next().is_none());
    assert_eq!(rebaser.rebased().len(), 1);

    tx.discard();
}

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_abandon(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit B and commit E were abandoned. Commit C and commit D should get
    // rebased onto commit A. Commit F should get rebased onto the new commit D.
    //
    // F
    // E
    // D C
    // |/
    // B
    // A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_d = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_e = graph_builder.commit_with_parents(&[&commit_d]);
    let commit_f = graph_builder.commit_with_parents(&[&commit_e]);

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {},
        hashset! {commit_b.id().clone(), commit_e.id().clone()},
    );
    assert_rebased(rebaser.rebase_next(), &commit_c, &[&commit_a]);
    let new_commit_d = assert_rebased(rebaser.rebase_next(), &commit_d, &[&commit_a]);
    assert_rebased(rebaser.rebase_next(), &commit_f, &[&new_commit_d]);
    assert!(rebaser.rebase_next().is_none());
    assert_eq!(rebaser.rebased().len(), 3);

    tx.discard();
}

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_abandon_and_replace(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit B was replaced by commit E. Commit C was abandoned. Commit D should
    // get rebased onto commit E.
    //
    //   D
    //   C
    // E B
    // |/
    // A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_d = graph_builder.commit_with_parents(&[&commit_c]);
    let commit_e = graph_builder.commit_with_parents(&[&commit_a]);

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {commit_b.id().clone() => hashset!{commit_e.id().clone()}},
        hashset! {commit_c.id().clone()},
    );
    assert_rebased(rebaser.rebase_next(), &commit_d, &[&commit_e]);
    assert!(rebaser.rebase_next().is_none());
    assert_eq!(rebaser.rebased().len(), 1);

    tx.discard();
}

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_abandon_degenerate_merge(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit B was abandoned. Commit D should get rebased to have only C as parent
    // (not A and C).
    //
    // D
    // |\
    // B C
    // |/
    // A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_d = graph_builder.commit_with_parents(&[&commit_b, &commit_c]);

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {},
        hashset! {commit_b.id().clone()},
    );
    assert_rebased(rebaser.rebase_next(), &commit_d, &[&commit_c]);
    assert!(rebaser.rebase_next().is_none());
    assert_eq!(rebaser.rebased().len(), 1);

    tx.discard();
}

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_abandon_widen_merge(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit E was abandoned. Commit F should get rebased to have B, C, and D as
    // parents (in that order).
    //
    // F
    // |\
    // E \
    // |\ \
    // B C D
    //  \|/
    //   A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_d = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_e = graph_builder.commit_with_parents(&[&commit_b, &commit_c]);
    let commit_f = graph_builder.commit_with_parents(&[&commit_e, &commit_d]);

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {},
        hashset! {commit_e.id().clone()},
    );
    assert_rebased(
        rebaser.rebase_next(),
        &commit_f,
        &[&commit_b, &commit_c, &commit_d],
    );
    assert!(rebaser.rebase_next().is_none());
    assert_eq!(rebaser.rebased().len(), 1);

    tx.discard();
}

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_multiple_sideways(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit B and commit D were both replaced by commit F. Commit C and commit E
    // should get rebased onto it.
    //
    // C E
    // B D F
    // | |/
    // |/
    // A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_d = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_e = graph_builder.commit_with_parents(&[&commit_d]);
    let commit_f = graph_builder.commit_with_parents(&[&commit_a]);

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {
            commit_b.id().clone() => hashset!{commit_f.id().clone()},
            commit_d.id().clone() => hashset!{commit_f.id().clone()},
        },
        hashset! {},
    );
    assert_rebased(rebaser.rebase_next(), &commit_c, &[&commit_f]);
    assert_rebased(rebaser.rebase_next(), &commit_e, &[&commit_f]);
    assert!(rebaser.rebase_next().is_none());
    assert_eq!(rebaser.rebased().len(), 2);

    tx.discard();
}

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_multiple_swap(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit B was replaced by commit D. Commit D was replaced by commit B.
    // Commit C and commit E should swap places.
    //
    // C E
    // B D
    // |/
    // A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_d = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_e = graph_builder.commit_with_parents(&[&commit_d]);

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {
            commit_b.id().clone() => hashset!{commit_d.id().clone()},
            commit_d.id().clone() => hashset!{commit_b.id().clone()},
        },
        hashset! {},
    );
    assert_rebased(rebaser.rebase_next(), &commit_c, &[&commit_d]);
    assert_rebased(rebaser.rebase_next(), &commit_e, &[&commit_b]);
    assert!(rebaser.rebase_next().is_none());
    assert_eq!(rebaser.rebased().len(), 2);

    tx.discard();
}

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_multiple_forward_and_backward(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit B was replaced by commit D. Commit F was replaced by commit C.
    // Commit G should be rebased onto commit C. Commit 8 should be rebased onto
    // commit D. Commits C-D should be left alone since they're ancestors of D.
    // Commit E should be left alone since its already in place (as a descendant of
    // D).
    //
    // G
    // F
    // E
    // D
    // C 8
    // |/
    // B
    // A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_d = graph_builder.commit_with_parents(&[&commit_c]);
    let commit_e = graph_builder.commit_with_parents(&[&commit_d]);
    let commit_f = graph_builder.commit_with_parents(&[&commit_e]);
    let commit_g = graph_builder.commit_with_parents(&[&commit_f]);
    let commit_h = graph_builder.commit_with_parents(&[&commit_b]);

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {
            commit_b.id().clone() => hashset!{commit_d.id().clone()},
            commit_f.id().clone() => hashset!{commit_c.id().clone()},
        },
        hashset! {},
    );
    assert_rebased(rebaser.rebase_next(), &commit_g, &[&commit_c]);
    assert_rebased(rebaser.rebase_next(), &commit_h, &[&commit_d]);
    assert!(rebaser.rebase_next().is_none());
    assert_eq!(rebaser.rebased().len(), 2);

    tx.discard();
}

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_divergent_rewrite(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit B was replaced by commit B2. Commit D was replaced by commits D2 and
    // D3. Commit F was replaced by commit F2. Commit C should be rebased onto
    // B2. Commit E should not be rebased. Commit F should be rebased onto
    // commit F2.
    //
    // G
    // F
    // E
    // D
    // C
    // B
    // | F2
    // |/
    // | D3
    // |/
    // | D2
    // |/
    // | B2
    // |/
    // A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_b]);
    let commit_d = graph_builder.commit_with_parents(&[&commit_c]);
    let commit_e = graph_builder.commit_with_parents(&[&commit_d]);
    let commit_f = graph_builder.commit_with_parents(&[&commit_e]);
    let commit_g = graph_builder.commit_with_parents(&[&commit_f]);
    let commit_b2 = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_d2 = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_d3 = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_f2 = graph_builder.commit_with_parents(&[&commit_a]);

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {
            commit_b.id().clone() => hashset!{commit_b2.id().clone()},
            commit_d.id().clone() => hashset!{commit_d2.id().clone(), commit_d3.id().clone()},
            commit_f.id().clone() => hashset!{commit_f2.id().clone()},
        },
        hashset! {},
    );
    assert_rebased(rebaser.rebase_next(), &commit_c, &[&commit_b2]);
    assert_rebased(rebaser.rebase_next(), &commit_g, &[&commit_f2]);
    assert!(rebaser.rebase_next().is_none());
    assert_eq!(rebaser.rebased().len(), 2);

    tx.discard();
}

#[test_case(false ; "local backend")]
#[test_case(true ; "git backend")]
fn test_rebase_descendants_contents(use_git: bool) {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, use_git);

    // Commit B was replaced by commit D. Commit C should have the changes from
    // commit C and commit D, but not the changes from commit B.
    //
    // D
    // | C
    // | B
    // |/
    // A
    let mut tx = repo.start_transaction("test");
    let path1 = RepoPath::from_internal_string("file1");
    let tree1 = testutils::create_tree(&repo, &[(&path1, "content")]);
    let commit_a = CommitBuilder::for_new_commit(&settings, repo.store(), tree1.id().clone())
        .write_to_repo(tx.mut_repo());
    let path2 = RepoPath::from_internal_string("file2");
    let tree2 = testutils::create_tree(&repo, &[(&path2, "content")]);
    let commit_b = CommitBuilder::for_new_commit(&settings, repo.store(), tree2.id().clone())
        .set_parents(vec![commit_a.id().clone()])
        .write_to_repo(tx.mut_repo());
    let path3 = RepoPath::from_internal_string("file3");
    let tree3 = testutils::create_tree(&repo, &[(&path3, "content")]);
    let commit_c = CommitBuilder::for_new_commit(&settings, repo.store(), tree3.id().clone())
        .set_parents(vec![commit_b.id().clone()])
        .write_to_repo(tx.mut_repo());
    let path4 = RepoPath::from_internal_string("file4");
    let tree4 = testutils::create_tree(&repo, &[(&path4, "content")]);
    let commit_d = CommitBuilder::for_new_commit(&settings, repo.store(), tree4.id().clone())
        .set_parents(vec![commit_a.id().clone()])
        .write_to_repo(tx.mut_repo());

    let mut rebaser = DescendantRebaser::new(
        &settings,
        tx.mut_repo(),
        hashmap! {
            commit_b.id().clone() => hashset!{commit_d.id().clone()}
        },
        hashset! {},
    );
    rebaser.rebase_all();
    let rebased = rebaser.rebased();
    assert_eq!(rebased.len(), 1);
    let new_commit_c = repo
        .store()
        .get_commit(rebased.get(commit_c.id()).unwrap())
        .unwrap();

    assert_eq!(
        new_commit_c.tree().path_value(&path3),
        commit_c.tree().path_value(&path3)
    );
    assert_eq!(
        new_commit_c.tree().path_value(&path4),
        commit_d.tree().path_value(&path4)
    );
    assert_ne!(
        new_commit_c.tree().path_value(&path2),
        commit_b.tree().path_value(&path2)
    );

    tx.discard();
}

#[test]
fn test_rebase_descendants_basic_branch_update() {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, false);

    // Branch "main" points to branch B. B gets rewritten as B2. Branch main should
    // be updated to point to B2.
    //
    // B main         B2 main
    // |         =>   |
    // A              A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    tx.mut_repo()
        .set_local_branch("main".to_string(), RefTarget::Normal(commit_b.id().clone()));
    tx.mut_repo().set_remote_branch(
        "main".to_string(),
        "origin".to_string(),
        RefTarget::Normal(commit_b.id().clone()),
    );
    tx.mut_repo()
        .set_tag("v1".to_string(), RefTarget::Normal(commit_b.id().clone()));
    let repo = tx.commit();

    let mut tx = repo.start_transaction("test");
    let commit_b2 = CommitBuilder::for_rewrite_from(&settings, repo.store(), &commit_b)
        .write_to_repo(tx.mut_repo());
    tx.mut_repo()
        .create_descendant_rebaser(&settings)
        .rebase_all();
    assert_eq!(
        tx.mut_repo().get_local_branch("main"),
        Some(RefTarget::Normal(commit_b2.id().clone()))
    );
    // The remote branch and tag should not get updated
    assert_eq!(
        tx.mut_repo().get_remote_branch("main", "origin"),
        Some(RefTarget::Normal(commit_b.id().clone()))
    );
    assert_eq!(
        tx.mut_repo().get_tag("v1"),
        Some(RefTarget::Normal(commit_b.id().clone()))
    );

    tx.discard();
}

#[test]
fn test_rebase_descendants_update_branches_after_divergent_rewrite() {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, false);

    // Branch "main" points to commit B. B gets rewritten as B2, B3, B4. Branch main
    // should become a conflict pointing to all of them.
    //
    //                B4 main?
    //                | B3 main?
    // B main         |/B2 main?
    // |         =>   |/
    // A              A
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    tx.mut_repo()
        .set_local_branch("main".to_string(), RefTarget::Normal(commit_b.id().clone()));
    let repo = tx.commit();

    let mut tx = repo.start_transaction("test");
    let commit_b2 = CommitBuilder::for_rewrite_from(&settings, repo.store(), &commit_b)
        .write_to_repo(tx.mut_repo());
    // Different description so they're not the same commit
    let commit_b3 = CommitBuilder::for_rewrite_from(&settings, repo.store(), &commit_b)
        .set_description("different".to_string())
        .write_to_repo(tx.mut_repo());
    // Different description so they're not the same commit
    let commit_b4 = CommitBuilder::for_rewrite_from(&settings, repo.store(), &commit_b)
        .set_description("more different".to_string())
        .write_to_repo(tx.mut_repo());
    tx.mut_repo()
        .create_descendant_rebaser(&settings)
        .rebase_all();
    assert_eq!(
        tx.mut_repo().get_local_branch("main"),
        Some(RefTarget::Conflict {
            removes: vec![commit_b.id().clone(), commit_b.id().clone()],
            adds: vec![
                commit_b2.id().clone(),
                commit_b3.id().clone(),
                commit_b4.id().clone()
            ]
        })
    );

    tx.discard();
}

#[test]
fn test_rebase_descendants_rewrite_updates_branch_conflict() {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, false);

    // Branch "main" is a conflict removing commit A and adding commit B and C.
    // A gets rewritten as A2 and A3. B gets rewritten as B2 and B2. The branch
    // should become a conflict removing A2, A3, and B, and adding A, B2, B3, C.
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.initial_commit();
    let commit_c = graph_builder.initial_commit();
    tx.mut_repo().set_local_branch(
        "main".to_string(),
        RefTarget::Conflict {
            removes: vec![commit_a.id().clone()],
            adds: vec![commit_b.id().clone(), commit_c.id().clone()],
        },
    );
    let repo = tx.commit();

    let mut tx = repo.start_transaction("test");
    let commit_a2 = CommitBuilder::for_rewrite_from(&settings, repo.store(), &commit_a)
        .write_to_repo(tx.mut_repo());
    // Different description so they're not the same commit
    let commit_a3 = CommitBuilder::for_rewrite_from(&settings, repo.store(), &commit_a)
        .set_description("different".to_string())
        .write_to_repo(tx.mut_repo());
    let commit_b2 = CommitBuilder::for_rewrite_from(&settings, repo.store(), &commit_b)
        .write_to_repo(tx.mut_repo());
    // Different description so they're not the same commit
    let commit_b3 = CommitBuilder::for_rewrite_from(&settings, repo.store(), &commit_b)
        .set_description("different".to_string())
        .write_to_repo(tx.mut_repo());
    tx.mut_repo()
        .create_descendant_rebaser(&settings)
        .rebase_all();
    assert_eq!(
        tx.mut_repo().get_local_branch("main"),
        Some(RefTarget::Conflict {
            removes: vec![
                commit_a2.id().clone(),
                commit_a3.id().clone(),
                commit_b.id().clone(),
            ],
            adds: vec![
                commit_c.id().clone(),
                commit_a.id().clone(),
                commit_b2.id().clone(),
                commit_b3.id().clone(),
            ]
        })
    );

    tx.discard();
}

#[test]
fn test_rebase_descendants_rewrite_resolves_branch_conflict() {
    let settings = testutils::user_settings();
    let (_temp_dir, repo) = testutils::init_repo(&settings, false);

    // Branch "main" is a conflict removing ancestor commit A and adding commit B
    // and C (maybe it moved forward to B locally and moved forward to C
    // remotely). Now B gets rewritten as B2, which is a descendant of C (maybe
    // B was automatically rebased on top of the updated remote). That
    // would result in a conflict removing A and adding B2 and C. However, since C
    // is a descendant of A, and B2 is a descendant of C, the conflict gets
    // resolved to B2.
    let mut tx = repo.start_transaction("test");
    let mut graph_builder = CommitGraphBuilder::new(&settings, tx.mut_repo());
    let commit_a = graph_builder.initial_commit();
    let commit_b = graph_builder.commit_with_parents(&[&commit_a]);
    let commit_c = graph_builder.commit_with_parents(&[&commit_a]);
    tx.mut_repo().set_local_branch(
        "main".to_string(),
        RefTarget::Conflict {
            removes: vec![commit_a.id().clone()],
            adds: vec![commit_b.id().clone(), commit_c.id().clone()],
        },
    );
    let repo = tx.commit();

    let mut tx = repo.start_transaction("test");
    let commit_b2 = CommitBuilder::for_rewrite_from(&settings, repo.store(), &commit_b)
        .set_parents(vec![commit_c.id().clone()])
        .write_to_repo(tx.mut_repo());
    tx.mut_repo()
        .create_descendant_rebaser(&settings)
        .rebase_all();
    assert_eq!(
        tx.mut_repo().get_local_branch("main"),
        Some(RefTarget::Normal(commit_b2.id().clone()))
    );

    tx.discard();
}

// TODO: Add a test for the following case, which can't happen with our current
// evolution-based rewriting.
//
// 1. Operation 1 points a branch to commit A
// 2. Operation 2 repoints the branch to commit B
// 3. Operation 3, which is concurrent with operation 2, deletes the branch
// 4. Resolved state (operation 4) will have a "-A+B" state for the branch
//
// Now we hide B and make A visible instead. When that diff is applied to the
// branch, the branch state becomes empty and is thus deleted.
