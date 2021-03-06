package it.casatta

import android.content.Intent
import androidx.test.espresso.Espresso
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.*
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.rule.ActivityTestRule
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith


@RunWith(AndroidJUnit4::class)
class SignTest : Common() {

    @get:Rule
    var activityRule: ActivityTestRule<MainActivity> = ActivityTestRule(
        MainActivity::class.java,
        true,
        false
    )

    @Test
    fun sign() {
        val activity = activityRule.launchActivity(Intent())
        val network = getNetwork()
        if ("mainnet" != network) {
            val aliceTprv = "tprv8ZgxMBicQKsPfEf2t9eG7j14CDjS3JWL9nY3wgg6ZsLKY4tsR4wZjYuLsXWdyBPrMPo73JgeKmbd8pTkZZgQNWTdvCtDuauf52XGKL9zTDw"
            val aliceKeyName = "alice_sign_test"
            val bobTprv = "tprv8ZgxMBicQKsPetwSbvkSob1PLvNeBzHftBgG61S37ywMpsCnKMkUhPbKp7FyZDsU2QvMqbF797DRqmwedPQnR5qqmUBkFVb7iNeKcEZv3ck"
            val bobKeyName = "bob_sign_test"
            val wallet = "{\n" +
                    "  \"name\": \"alice-and-bob\",\n" +
                    "  \"descriptor\": \"wsh(multi(2,tpubD6NzVbkrYhZ4YhgpmoJrX8fAmFFNCdhEj68qECiPz98iNZ9e3Tm9v3XD3fzHZfBoLqeSm9oLtighoeijQ9jGAFm9raQ4JqHZ1N4BHyaBz6Y/0/*,tpubD6NzVbkrYhZ4YMyEVaR3CzfVuwtaMKUaTVH3NXULYFjkfMTYwka4stDBzHhHkxd4MEMVgyyEV1WBCrpwde72w8LzjAE6oRLARBAiCD8cGQV/0/*))#wss3kl0z\",\n" +
                    "  \"fingerprints\": [\n" +
                    "    \"1f5e43d8\",\n" +
                    "    \"a2ebe04e\"\n" +
                    "  ],\n" +
                    "  \"required_sig\": 2,\n" +
                    "  \"created_at_height\": 1835680\n" +
                    "}\n"
            val walletName = "alice-and-bob"

            val tx = "cHNidP8BAFMCAAAAASFSbAAqstjwTxbGtWir21+meBp5LMcUQsBSgZ5bDtD7AQAAAAD+////AV6rCAAAAAAAF6kU4wEfjwloN3dvCV9wNOekdO53E92HAAAAAAX8bmFtZQh0by1jYXJvbAABAKECAAAAAcyd+J9zW1wSNV/mozPMv8mcXFzwQrK1EKq/FvRPJS40AQAAACMiACC+U25ZjJg9CiGsPhlAqQ0GWtFhOWxqopXdDTrh2oBdEP3///8Cp0lVAAAAAAAXqRRUIuqRoByuLh5D6zdViHWG7aGi84cVrAgAAAAAACIAIDz80EGjAUinXjMddGAtfQ3fKqcjgWj9wY5Y+8c7NA1zoAIcAAEBKxWsCAAAAAAAIgAgPPzQQaMBSKdeMx10YC19Dd8qpyOBaP3Bjlj7xzs0DXMBBUdSIQNP26ruccaqcu2cxRFYsPON2gj4ALrAFQ5ApBVtM+z9SiECIwjICs3MMHNnGbXPgSQKezAcOC5HzejKyjATzR8qXiRSriIGAiMIyArNzDBzZxm1z4EkCnswHDguR83oysowE80fKl4kDB9eQ9gAAAAAAAAAACIGA0/bqu5xxqpy7ZzFEViw843aCPgAusAVDkCkFW0z7P1KDKLr4E4AAAAAAAAAAAAA"
            val txName = "to-carol"

            /// START importing key, wallet and tx
            onView(withId(R.id.key_button)).perform(click())

            onView(withId(R.id.item_new)).perform(click())
            clickElementInList("Import tprv")
            setTextInDialogAndConfirm(activity, aliceKeyName)
            setTextInDialogAndConfirm(activity, aliceTprv)

            onView(withId(R.id.item_new)).perform(click())
            clickElementInList("Import tprv")
            setTextInDialogAndConfirm(activity, bobKeyName)
            setTextInDialogAndConfirm(activity, bobTprv)

            Espresso.pressBack()

            onView(withId(R.id.wallet_button)).perform(click())
            onView(withId(R.id.item_new)).perform(click())
            clickElementInList(activity.getString(R.string.insert_manually))
            setTextInDialogAndConfirm(activity, wallet)

            Espresso.pressBack()

            onView(withId(R.id.psbt_button)).perform(click())
            onView(withId(R.id.item_new)).perform(click())
            clickElementInList(activity.getString(R.string.insert_manually))
            setTextInDialogAndConfirm(activity, tx)

            Espresso.pressBack()
            /// END importing key, wallet and tx

            /// START selecting key, wallet and tx
            onView(withId(R.id.key_button)).perform(click())
            clickElementInList(aliceKeyName)
            onView(withId(R.id.select)).perform(click())

            onView(withId(R.id.wallet_button)).perform(click())
            clickElementInList(walletName)
            onView(withId(R.id.select)).perform(click())

            onView(withId(R.id.psbt_button)).perform(click())
            clickElementInList(txName)
            onView(withId(R.id.select)).perform(click())
            /// END selecting key, wallet and tx

            /// START signing
            onView(withId(R.id.sign_button)).perform(click())
            checkAndDismissDialog(R.string.added_signatures)

            onView(withId(R.id.sign_button)).perform(click())
            checkAndDismissDialog("request to sign a PSBT already containing a signature from this key")

            onView(withId(R.id.key_button)).perform(click())
            clickElementInList(bobKeyName)
            onView(withId(R.id.select)).perform(click())

            onView(withId(R.id.sign_button)).perform(click())
            checkAndDismissDialog(R.string.added_signatures)

            onView(withId(R.id.sign_button)).perform(click())
            checkAndDismissDialog("request to sign a PSBT already containing a signature from this key")
            /// END signing

            /// START deleting key, wallet and tx
            onView(withId(R.id.psbt_button)).perform(click())
            clickElementInList(txName)
            onView(withId(R.id.delete)).perform(click())
            setTextInDialogAndConfirm(activity, txName, "DELETE")
            checkAndDismissDialog(R.string.deleted)

            onView(withId(R.id.wallet_button)).perform(click())
            clickElementInList(walletName)
            onView(withId(R.id.delete)).perform(click())
            setTextInDialogAndConfirm(activity, walletName, "DELETE")
            checkAndDismissDialog(R.string.deleted)

            onView(withId(R.id.key_button)).perform(click())
            clickElementInList(aliceKeyName)
            onView(withId(R.id.delete)).perform(click())
            setTextInDialogAndConfirm(activity, aliceKeyName, "DELETE")
            checkAndDismissDialog(R.string.deleted)

            onView(withId(R.id.key_button)).perform(click())
            clickElementInList(bobKeyName)
            onView(withId(R.id.delete)).perform(click())
            setTextInDialogAndConfirm(activity, bobKeyName, "DELETE")
            checkAndDismissDialog(R.string.deleted)
            /// END deleting key, wallet and tx
        }
    }


}
