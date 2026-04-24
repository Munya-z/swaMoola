The Match: 
Person A in South Africa (SA) wants to send R50 to Zim.
Person B in Zimbabwe (Zim) wants to send $3 to SA.

The Swap:
Person A gives R50 to Person B’s contact who is already in SA.
Person B gives $3 to Person A’s family already in Zim.

The App's Role:
It tracks these "IOUs," verifies that both
sides actually paid, and uses a rating/trust system
(like Uber or Airbnb) so you know who is reliable.

3. Community-Based Trust (The Referral Loop)
The strongest trust in migrant communities isn't
a government ID; it’s who you know 

Vouching System: A new user cannot join unless 
a "Verified" member invites them 

Shared Risk: If a new person scams someone, the
person who invited them also loses "Trust Points."
This makes people very careful about who they bring into the app 


4. Data Minimisation Checklist
When building the app, follow these "less is more" rules 

No Location Tracking: Instead of GPS,
let users manually select a "General Area" 
(e.g., "Sandton" or "Bulawayo Central") 

Auto-Delete History: Delete transaction details
after 30 days. If the swap is finished and everyone
is happy, there is no reason for you to keep that data forever 

No Social Media Login: Avoid "Login with Facebook/Google."
These services track everything your users do.
Use a simple Username/Password or Email 

5. Escrow (The "Safety Net")
To solve the "Who goes first?" problem without 
holding personal data, use a Digital Handshake:
Both parties must "Check-in" at a location (or enter a code)
before the trust score is updated.
The app can act as a neutral witness that confirms:
"User A says they paid, User B says they received."

1. The "Code Pass-Down" Workflow
The Match: Sender A (in SA) and Sender B (in Zim) 
agree to the swap on the app.
The App Generates Codes: The app gives Sender A a 
secret code and Sender B a secret code.
The Hand-Off:
Sender A sends their code to their relative (the Recipient) 
in Zimbabwe via WhatsApp or SMS.
Sender B sends their code to their relative in South Africa.
The Physical Swap:
When the relatives meet, the relative receiving the cash 
must provide the code to the person handing it over.
Example: Relative in Zim gives the cash to Relative A; 
Relative A then gives the code to Relative B.
Closing the Loop: The relatives then tell the Senders 
(the app users) that the trade is done. The Senders 
enter the codes into the app to confirm.
2. How to handle the "No App" Recipient
Since the recipients don't have the app, the 
Senders act as the "Verification Officers."
Trust on the Senders: All the "Trust Score" 
points go to the Senders. If a recipient causes 
rouble (e.g., tries to use a fake code), the 
Sender’s account is the one that gets flagged 
or banned. This forces Senders to make sure their 
relatives understand how the system works.
Simple Verification: Because there is no app for 
the recipient to scan, the code needs to be short 
and easy to read (like a 4 or 5-digit PIN).
3. The Biggest Risk: The "Double-Cross"
Since the app isn't "watching" the exchange via 
GPS or a live scan, a Sender could lie and say, 
"I gave the money, but they never gave me the code," even if they did.
To prevent this without taking more data, you 
an use Escrow Milestones:
Stage 1: Sender A and Sender B both "Commit" to the trade.
Stage 2: The codes are revealed to the Senders.
Stage 3: The app waits for both Senders to enter the 
codes they received from their relatives.
If only one person enters a code: The app flags 
the transaction. The person who didn't enter a code 
has 24 hours to explain why, or their Trust Score drops 
to zero and they are blocked from future trades.
4. Keeping it Anonymous (The "No Data" Goal)
Since you only care about the Senders, you can 
stay very private:
No Recipient Names: The app never asks for the names 
of the family members. It just says "Recipient in Zim."
No Recipient Numbers: The Senders share the codes via 
their own private WhatsApp/SMS. Your app never sees 
that conversation.
Encrypted Codes: The codes are only stored on your 
server as "hashes" (jumbled text), so even if you were 
hacked, no one could steal a "live" code to go and scam a family member.
The "Community Voucher" Idea
Since you want to keep data to a minimum, you could 
even make the account creation based on a physical vouch. 
A new Sender joins by using a code from a trusted Sender. 
This creates a "network of accountability" where people only 
rade with people who are "friends of friends."

4. Safety within the Chat
Since you aren't collecting much personal data, 
the chat needs to protect users:
System Warnings: If someone types words like "Bank Account," 
"Password," or "WhatsApp me," the app can show a pop-up warning: 
"Stay safe: Keep all agreements inside the app to protect 
your Trust Score."
No Media: To keep the app "light" and avoid people sending 
inappropriate or heavy files, you can make the chat text-only.

The "Trust" Connection
The chat is actually a great way to build the 
Trust Score you mentioned earlier.
If a user is polite and quick to respond in chat, 
the other person can give them a "Good Communication" badge.
If someone agrees to a rate in chat but then tries to 
change it in person (a common tactic), the victim 
can report them, and that user's Trust Score will tank.

Integrating EFTs (Electronic Funds Transfers) or mobile 
money (like EcoCash or Mukuru) changes the game because 
the "handover" isn't happening in person, but the 
pre-generated code system can still be your best friend.
Since you want to keep the app "light" on data, here is 
how you can handle digital payments without becoming a bank yourself:
1. The "Digital Receipt" Handshake
If the parties aren't meeting physically, the code is 
exchanged digitally once the money reflects.
The Process:
Sender A in SA sends an EFT to Sender B’s relative.
Sender B’s relative checks their bank app. Once they 
see the "Available Balance" has increased, they send 
the Secret Code (the one they got from Sender B) to 
Sender A’s relative via WhatsApp.
Sender A’s relative gives that code to Sender A, who 
enters it into your app to finish the trade.
The Rule: The app must remind users: "Never share 
the code until you see the money in your ACTUAL bank 
app." (To avoid "fake SMS" payment scams).
2. Proof of Payment (PoP) Upload
To add a layer of security for EFTs without collecting 
much data, you can add a "Upload Screenshot" feature in the chat.
The person sending the EFT takes a screenshot of the 
"Success" screen and sends it in the chat.
Privacy Tip: Your app can automatically "blur" parts 
of the photo or remind the user to crop out their bank 
balance or account number before sending, so only the 
Transaction Reference and Amount are visible.
3. The "Escrow" Level-Up (Optional)
EFTs are riskier than cash because you can't "take back" 
a payment if the other person doesn't give you the code. 
To handle this, you could introduce Status Tags:
"Verified EFT Users": Only people with a very high 
Trust Score (e.g., 50+ successful cash trades) are 
allowed to toggle the "Open to EFT" button on their profile.
This protects new users from getting scammed while 
allowing "vetted" community members to trade digitally.
4. Handling Reversals
The biggest headache with EFTs is payment reversals. 
A scammer sends money, gets the code, and then calls 
their bank to cancel the transfer.
The "Slow Release" Trust Score: If a trade is done via 
EFT, the app doesn't give the "Trust Points" immediately. 
It waits 48 hours (the time it usually takes for a 
payment to become "permanent") before updating the score.
If a user reverses a payment, they are instantly banned, 
and their "Voucher" (the person who invited them) gets a 
warning or a penalty.
5. Neutral Locations
For the cash users, suggesting Neutral Locations is a 
great "safety-first" feature. You could have a simple 
list on the home screen:
"Public Malls"
"Fuel Stations (Forecourts)"
"Near Police Stations"
"Fast Food Outlets (Chicken Licken/Hungry Lion)"

Since you're thinking about EFTs, would you 
want the app to suggest which banks are 'instant' 
(e.g., Capitec to Capitec) to help users avoid the 2-day waiting period?
