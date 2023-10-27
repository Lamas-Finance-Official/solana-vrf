use anchor_lang::{prelude::*, Discriminator};
use vrf_sdk::{declare_vrf_state, vrf::VrfAccountData};

declare_id!("3gfec8ANuaWzkNhAR5QRjUvGqUjMYLJ3YnSVhgMkugqv");

declare_vrf_state!(VrfState);

#[test]
fn test_discriminator() {
    assert_eq!(
        <VrfState as Discriminator>::DISCRIMINATOR,
        VrfAccountData::DISCRIMINATOR
    );
}
