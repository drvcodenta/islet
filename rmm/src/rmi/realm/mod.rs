pub(crate) mod params;

use self::params::Params;
use super::error::Error;
use crate::event::Mainloop;
use crate::granule::GRANULE_SIZE;
use crate::granule::{set_granule, GranuleState};
use crate::host;
use crate::listen;
use crate::measurement::{HashContext, MEASUREMENTS_SLOT_RIM};
use crate::mm::translation::PageTable;
use crate::realm::mm::stage2_translation::Stage2Translation;
use crate::realm::mm::IPATranslation;
use crate::realm::rd::State;
use crate::realm::rd::{insert_rtt, Rd};
use crate::realm::registry::{remove, VMID_SET};
use crate::rmi::{self, metadata::IsletRealmMetadata};
use crate::{get_granule, get_granule_if};

use alloc::boxed::Box;
use alloc::sync::Arc;
use spin::mutex::Mutex;

extern crate alloc;

pub fn set_event_handler(mainloop: &mut Mainloop) {
    listen!(mainloop, rmi::REALM_ACTIVATE, |arg, _, _| {
        let rd = arg[0];

        let mut rd_granule = get_granule_if!(rd, GranuleState::RD)?;
        let mut rd = rd_granule.content_mut::<Rd>()?;

        if let Some(meta) = rd.metadata() {
            info!("Realm metadata is in use!");
            let g_metadata = get_granule_if!(meta, GranuleState::Metadata)?;
            let metadata = g_metadata.content::<IsletRealmMetadata>()?;

            if !metadata.equal_rd_rim(&rd.measurements[MEASUREMENTS_SLOT_RIM]) {
                error!("Calculated rim and those read from metadata are not the same!");
                return Err(Error::RmiErrorRealm(0));
            }

            if !metadata.equal_rd_hash_algo(rd.hash_algo()) {
                error!("Provided measurement hash algorithm and metadata hash algorithm are different!");
                return Err(Error::RmiErrorRealm(0));
            }
        }

        if !rd.at_state(State::New) {
            return Err(Error::RmiErrorRealm(0));
        }

        rd.set_state(State::Active);
        Ok(())
    });

    listen!(mainloop, rmi::REALM_CREATE, |arg, _, rmm| {
        let rd = arg[0];
        let params_ptr = arg[1];

        if rd == params_ptr {
            return Err(Error::RmiErrorInput);
        }

        let mut rd_granule = get_granule_if!(rd, GranuleState::Delegated)?;
        let mut rd_obj = rd_granule.content_mut::<Rd>()?;
        #[cfg(not(kani))]
        // `page_table` is currently not reachable in model checking harnesses
        rmm.page_table.map(rd, true);

        let params = host::copy_from::<Params>(params_ptr).ok_or(Error::RmiErrorInput)?;
        params.verify_compliance(rd)?;

        let rtt_granule = get_granule_if!(params.rtt_base as usize, GranuleState::Delegated)?;
        // This is required to prevent from the deadlock in the below epilog
        // which acquires the same lock again
        core::mem::drop(rtt_granule);

        // revisit rmi.create_realm() (is it necessary?)
        create_realm(params.vmid as usize).map(|_| {
            let s2 = Box::new(Stage2Translation::new(
                params.rtt_base as usize,
                params.rtt_level_start as usize,
                params.rtt_num_start as usize,
            )) as Box<dyn IPATranslation>;

            insert_rtt(params.vmid as usize, Arc::new(Mutex::new(s2)));

            rd_obj.init(
                params.vmid,
                params.rtt_base as usize,
                params.ipa_bits(),
                params.rtt_level_start as isize,
                params.rpv,
            )
        })?;

        let rtt_base = rd_obj.rtt_base();
        // The below is added to avoid a fault regarding the RTT entry
        for i in 0..params.rtt_num_start as usize {
            let rtt = rtt_base + i * GRANULE_SIZE;
            PageTable::get_ref().map(rtt, true);
        }

        rd_obj.set_hash_algo(params.hash_algo);

        #[cfg(not(kani))]
        // `rsi` is currently not reachable in model checking harnesses
        HashContext::new(&mut rd_obj)?.measure_realm_create(&params)?;

        let mut eplilog = move || {
            let mut rtt_granule = get_granule_if!(rtt_base, GranuleState::Delegated)?;
            set_granule(&mut rtt_granule, GranuleState::RTT)?;
            set_granule(&mut rd_granule, GranuleState::RD)
        };

        eplilog().map_err(|e| {
            #[cfg(not(kani))]
            // `page_table` is currently not reachable in model checking harnesses
            rmm.page_table.unmap(rd);
            remove(params.vmid as usize).expect("Realm should be created before.");
            e
        })
    });

    listen!(mainloop, rmi::REC_AUX_COUNT, |arg, ret, _| {
        let _ = get_granule_if!(arg[0], GranuleState::RD)?;
        ret[1] = rmi::MAX_REC_AUX_GRANULES;
        Ok(())
    });

    listen!(mainloop, rmi::REALM_DESTROY, |arg, _ret, rmm| {
        // get the lock for Rd
        let mut rd_granule = get_granule_if!(arg[0], GranuleState::RD)?;
        #[cfg(feature = "gst_page_table")]
        if rd_granule.num_children() > 0 {
            return Err(Error::RmiErrorRealm(0));
        }
        let mut rd = rd_granule.content::<Rd>()?;
        let vmid = rd.id();

        if let Some(meta) = rd.metadata() {
            let mut meta_granule = get_granule_if!(meta, GranuleState::Metadata)?;
            set_granule(&mut meta_granule, GranuleState::Delegated)?;
            rd.set_metadata(None);
        }

        let mut rtt_granule = get_granule_if!(rd.rtt_base(), GranuleState::RTT)?;
        #[cfg(feature = "gst_page_table")]
        if rd_granule.num_children() > 0 {
            return Err(Error::RmiErrorRealm(0));
        }
        set_granule(&mut rtt_granule, GranuleState::Delegated)?;

        // change state when everything goes fine.
        set_granule(&mut rd_granule, GranuleState::Delegated)?;
        #[cfg(not(kani))]
        // `page_table` is currently not reachable in model checking harnesses
        rmm.page_table.unmap(arg[0]);
        remove(vmid)?;

        Ok(())
    });

    listen!(
        mainloop,
        rmi::ISLET_REALM_SET_METADATA,
        |arg, _ret, _rmm| {
            let rd_addr = arg[0];
            let mdg_addr = arg[1];
            let meta_ptr = arg[2];

            // TODO: should we really hold a whole 4k on the stack? Either remove
            // the unused field from metadata or copy directly from granule to
            // granule. Is there any use for the unused? Its content is irrelevant.
            let realm_metadata = IsletRealmMetadata::from_ns(meta_ptr)?;
            realm_metadata.dump();

            if let Err(e) = realm_metadata.verify_signature() {
                error!("Verification of realm metadata signature has failed");
                Err(e)?;
            }

            if let Err(e) = realm_metadata.validate() {
                error!("The content of realm metadata is not valid");
                Err(e)?;
            }

            let mut rd_granule = get_granule_if!(rd_addr, GranuleState::RD)?;
            let mut rd = rd_granule.content_mut::<Rd>()?;
            if rd.metadata().is_some() {
                error!("Metadata is already set");
                Err(Error::RmiErrorRealm(0))?;
            }

            let mut g_metadata = get_granule_if!(mdg_addr, GranuleState::Delegated)?;
            let mut meta = g_metadata.content_mut::<IsletRealmMetadata>()?;
            *meta = realm_metadata.clone();
            // TODO: with_parent?
            set_granule(&mut g_metadata, GranuleState::Metadata)?;

            rd.set_metadata(Some(mdg_addr));

            Ok(())
        }
    );
}

fn create_realm(vmid: usize) -> Result<(), Error> {
    let mut vmid_set = VMID_SET.lock();
    if vmid_set.contains(&vmid) {
        return Err(Error::RmiErrorInput);
    } else {
        vmid_set.insert(vmid);
    };

    Ok(())
}
