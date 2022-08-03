use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use frame_support::BoundedVec;
use frame_support::traits::ConstU32;


#[test]
fn round_control_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(TemplateModule::start_round(Origin::signed(1)));
		//someone else tryying to end
		assert_noop!(TemplateModule::end_round(Origin::signed(10)),Error::<Test>::NotAdmin);
		assert_ok!(TemplateModule::end_round(Origin::signed(1)));
	});
}

#[test]
fn create_proposal_works() {
	new_test_ext().execute_with(|| {
		let round_no = 1;
		let project_name : BoundedVec<u8, ConstU32<32>> = (vec![1u8,32]).try_into().unwrap();
		assert_ok!(TemplateModule::start_round(Origin::signed(1)));
		assert_ok!(TemplateModule::create_proposal(Origin::signed(1), project_name.clone(), round_no));
	});
}

#[test]
fn vote_works() {
	new_test_ext().execute_with(|| {
		let round_no = 1;
		let project_name : BoundedVec<u8, ConstU32<32>> = (vec![1u8,32]).try_into().unwrap();
		assert_ok!(TemplateModule::start_round(Origin::signed(1)));
		assert_ok!(TemplateModule::create_proposal(Origin::signed(1), project_name.clone(), round_no));
		//assert_ok!(TemplateModule::vote(Origin::signed(10), round_no, 1, 100));
		assert_ok!(TemplateModule::end_round(Origin::signed(1)));
	});
}